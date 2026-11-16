//! Adapter for Ely GPUI Components (`use: ely: ely-core` → `ely.*` elements).

use ely_core::buttons::{
    ActionBar, ActionSheet, BulkActionBar, Button, ButtonGroup, ButtonVariant, ConfirmButton,
    ConfirmMode, CopyButton, FloatingActionButton, IconButton, QuickAction, QuickActions,
    SegmentedControl, ShareButton, ToggleButton, ToggleGroup, ToggleItem,
};
use ely_core::data_display::{
    Activity, ActivityFeed, Avatar, AvatarGroup, Badge, Barcode, BeforeAfter, Carousel, Change,
    Changelog, Comparison, CountBadge, DescriptionList, DotBadge, Gallery, Gauge, KpiCard, Meter,
    PropertyGrid, PropertyGroup, QrCode, Release, Sparkline, Stars, Statistic, Tag, Timeline,
    TimelineItem, Tone, TrendIndicator, UsageBar, UserChip, Watermark,
};
use ely_core::feedback::{
    Alert, Banner, Callout, ConfirmationCard, ConnectionStatus, Countdown, EmptyState,
    ErrorBoundary, ErrorView, InlineMessage, Notification, NotificationCenter, ResultView,
    SaveState, SavingIndicator, StatusMessage, Suspense, SyncState, SyncStatus, Timer, Toast,
    ToastViewport, Toaster,
};
use ely_core::forms::{
    AvatarUpload, COUNTRIES, Calendar, Cascade, Cascader, Checkbox, CheckboxCard, CheckboxGroup,
    Choice, ChoiceChips, CodeInput, ColorPicker, ColorSwatch, Combobox, CronEditor, DatePicker,
    DateRangePicker, DateTimePicker, DirtyIndicator, DropZone, DurationPicker, EmailInput,
    EmojiPicker, ExpressionInput, EyeDropper, FieldArray, FileInput, FontPicker, Form, FormError,
    FormField, FormLabel, FormSection, GradientEditor, GradientStop, HotkeyInput, IconPicker,
    InlineEdit, InlineForm, Input, InputAddon, InputEvent, InputGroup, JsonInput, KeyValueInput,
    Knob, ListBox, ListInput, MaskedInput, MentionInput, MonthPicker, MultiSelect, NumberInput,
    PasswordInput, PathInput, PhoneInput, PinInput, QuarterPicker, Radio, RadioCard, RadioGroup,
    RangeSlider, Rating, RegexInput, RelativeDatePicker, RelativeRange, ScrubInput, SearchInput,
    Select, SignaturePad, Slider, Stepper, Switch, TagInput, TextInput, TimePicker, TimezoneSelect,
    TransferList, UnitInput, Upload, UploadList, UploadState, UrlInput, WeekPicker, YearPicker,
    hex as ely_hex, parse_hex,
};
use ely_core::layout::{
    Accordion, AccordionItem, AppShell, AspectRatio, Card, CardHeader, Container, Dock, DockPanel,
    DockSide, Drawer, FloatingPanel, Frame, Masonry, MasterDetail, Page, PaneGroup, PaneId,
    ScrollArea, ScrollShadow, ScrollToTop, SimpleGrid, SplitPane, StickyHeader, Viewport, Well,
    ZStack,
};
use ely_core::layout::{Collapsible, Fieldset, Panel, Section, Sheet, Sidebar};
use ely_core::lists::{
    DirEntry, DirectoryListing, FileTree, GitStatus, GroupedList, InfiniteList, List, ListItem,
    Outline, Sections, SelectableList, SortableList, SwipeAction, SwipeableListItem, Tree,
    TreeNode, TreeSelect, VirtualList,
};
use ely_core::menus::{
    ContextMenu, DropdownMenu, Menu, MenuItem, OverflowMenu, PieItem, PieMenu, SearchableMenu,
    SplitButton,
};
use ely_core::motion::{ProgressBar, Reorder, Skeleton};
use ely_core::motion::{Spinner, SpinnerStyle};
use ely_core::navigation::{
    Anchor, BackForwardNavigation, Breadcrumb, Crumb, EditorTab, EditorTabs, GoToLine, LoadMore,
    NavGroup, NavItem, NavigationMenu, Pagination, Sections as TocSections, Steps, TabOverflowMenu,
    TabPlacement, TableOfContents, Tabs, Wizard,
};
use ely_core::overlays::{
    AlertDialog, ConfirmDialog, Dialog, FloatingToolbar, HoverCard, Lightbox, Peek, Popover,
    PromptDialog, Slide,
};
use ely_core::primitives::Disclosure;
use ely_core::primitives::{Divider, Icon, IconName, Place, Severity};
use ely_core::shell::{
    ActivityBar, ActivityItem, ColumnShell, Connectivity, CrashReporter, OfflineIndicator,
    ResizeBorder, ShellColumn, StatusBar, StatusBarItem, TabBar, TitleBar, Toolbar, ToolbarGroup,
    ToolbarSeparator, UpdateBanner, UpdateDialog, UpdateState, Vibrancy, WindowControls,
    WindowSwitcher, WindowTab, ZoomControl, ZoomStep,
};
use ely_core::tables::{
    Aggregate, Cell as TableCell, Column, DataGrid, DataTable, HeatmapTable, PivotTable, Row,
    Spreadsheet, Table, TableToolbar, TreeRow, TreeTable,
};
use ely_core::theme::AvatarSize;
use ely_core::theme::Density;
use ely_core::theme::Platform;
use ely_core::theme::{ContainerSize, ControlSize, IconSize, TextSize};
use ely_core::typography::{
    AnimatedNumber, Blockquote, CurrencyText, DateTimeText, DurationStyle, DurationText, Ellipsis,
    EllipsisTooltip, Emoji, ExternalLink, FileSizeText, GradientText, Highlight, Kbd, KbdCombo,
    Link, MiddleEllipsis, PluralText, RelativeTime, SelectableText, ShimmerText, Typewriter,
};
use ely_core::typography::{Caption, Heading, Label, Overline, Paragraph, Subtitle, Title};
use ely_core::typography::{Code, CopyableText, NumberText, PercentText};
use jiff::Timestamp;
use jiff::civil::{Date, DateTime, Time};
use serde_json::Value as Json;
use std::time::Duration;
use web_time::Instant;

use gpui_forge_runtime::base::kit::*;
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Path, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;

/// `ely.*` adapter — register with `view.register_library(Ely)`.
///
/// Installs `ely_core::theme::Theme` on first render — widgets read it through
/// `ActiveTheme` and panic when the global is missing (`ely_core::init` also
/// loads Ely's asset bundle, which hosts may not ship).
pub struct Ely {
    inited: Cell<bool>,
    /// `&Entity<TextInput>` fields keyed by element key — inputs carry their
    /// state in the entity, so it must persist across renders; the view's own
    /// entity maps are private to the base.
    texts: RefCell<HashMap<String, Entity<ely_core::forms::TextInput>>>,
    tocs: RefCell<HashMap<String, Entity<TocSections>>>,
    /// Lazy child-content proxies keyed `{key}/{i}` — children render in the
    /// proxy's own pass, not inside ForgeView's (which would double-borrow).
    views: RefCell<HashMap<String, AnyView>>,
    /// One `Entity<Toaster>` per `ToastViewport` element key.
    toasters: RefCell<HashMap<String, Entity<Toaster>>>,
    /// Shared scroll handles — `handle:` props link `ScrollArea` to
    /// `ScrollShadow`/`ScrollToTop`/`StickyHeader`.
    handles: RefCell<HashMap<String, ScrollHandle>>,
    /// `Entity<PaneGroup>` and `Entity<Dock>` per element key.
    panes: RefCell<HashMap<String, Entity<PaneGroup>>>,
    docks: RefCell<HashMap<String, Entity<Dock>>>,
}

/// Renders one Forge child in its own entity update — the ely widgets'
/// lazy-content slots (`content` closures, `over`, pane bodies) need a view
/// they can clone and draw later.
struct LibProxy {
    view: WeakEntity<ForgeView>,
    node: Node,
    key: String,
}

impl Render for LibProxy {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.view
            .update(cx, |view, cx| {
                view.render_node(&self.node, &self.key, window, cx)
            })
            .unwrap_or_else(|_| div().into_any_element())
    }
}

impl Ely {
    pub fn new() -> Self {
        Self {
            inited: Cell::new(false),
            texts: RefCell::new(HashMap::new()),
            tocs: RefCell::new(HashMap::new()),
            views: RefCell::new(HashMap::new()),
            toasters: RefCell::new(HashMap::new()),
            handles: RefCell::new(HashMap::new()),
            panes: RefCell::new(HashMap::new()),
            docks: RefCell::new(HashMap::new()),
        }
    }

    /// The shared `Entity<TextInput>` behind `Input`/`SearchInput`/`MaskedInput`
    /// and friends — one per element key.
    fn text_input(
        &self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<ForgeView>,
    ) -> Entity<TextInput> {
        let mut texts = self.texts.borrow_mut();
        if let Some(t) = texts.get(key) {
            return t.clone();
        }
        let t = cx.new(|cx| TextInput::new(window, cx));
        texts.insert(key.to_owned(), t.clone());
        t
    }

    /// The shared `navigation::Sections` entity behind TableOfContents/Anchor —
    /// `toc` props name which one siblings share (default: the node's own key).
    fn toc(&self, name: &str, cx: &mut Context<ForgeView>) -> Entity<TocSections> {
        let mut tocs = self.tocs.borrow_mut();
        if let Some(t) = tocs.get(name) {
            return t.clone();
        }
        let t = cx.new(|_| TocSections::new(ScrollHandle::new()));
        tocs.insert(name.to_owned(), t.clone());
        t
    }

    /// A seeded, wired `Entity<TextInput>`: placeholder/value/disabled/masked
    /// applied once, `{{binding}}`/`on change:`/`on submit:` via a subscription.
    fn field(
        &self,
        node: &Node,
        key: &str,
        view: &ForgeView,
        window: &mut Window,
        cx: &mut Context<ForgeView>,
    ) -> Entity<TextInput> {
        let fresh = !self.texts.borrow().contains_key(key);
        let state = self.text_input(key, window, cx);
        if fresh {
            let placeholder = prop_str_opt(node, "placeholder");
            let text = prop_str(node, "value");
            let masked = prop_bool(node, "masked");
            let disabled = prop_bool(node, "disabled");
            state.update(cx, |s, cx| {
                if let Some(p) = placeholder {
                    s.set_placeholder(p, cx);
                }
                if !text.is_empty() {
                    s.set_text(text, cx);
                }
                if masked {
                    s.set_masked(true, cx);
                }
                if disabled {
                    s.set_disabled(true, cx);
                }
            });
            let path = binding_of(node, "value");
            let changes = calls_for(view, node, "change");
            let submits = calls_for(view, node, "submit");
            if path.is_some() || !changes.is_empty() || !submits.is_empty() {
                cx.subscribe(
                    &state,
                    move |view, entity, event: &InputEvent, cx| match event {
                        InputEvent::Changed => {
                            let text = entity.read(cx).text().to_string();
                            if let Some(path) = &path {
                                view.update_state(path, Json::String(text.clone()), cx);
                            }
                            for (name, args) in &changes {
                                let mut a = args.clone();
                                a.push(text.clone());
                                view.fire_call(name, a, cx);
                            }
                        }
                        InputEvent::Submit => {
                            let text = entity.read(cx).text().to_string();
                            for (name, args) in &submits {
                                let mut a = args.clone();
                                a.push(text.clone());
                                view.fire_call(name, a, cx);
                            }
                        }
                        _ => {}
                    },
                )
                .detach();
            }
        }
        state
    }

    /// A proxy view rendering `child` in a later pass — safe to hand to lazy
    /// content slots.
    fn child_view(&self, child: &Node, key: &str, cx: &mut Context<ForgeView>) -> AnyView {
        let mut views = self.views.borrow_mut();
        if let Some(v) = views.get(key) {
            return v.clone();
        }
        let node = child.clone();
        let k = key.to_owned();
        let weak = cx.entity().downgrade();
        let v: AnyView = cx
            .new(|_| LibProxy {
                view: weak,
                node,
                key: k,
            })
            .into();
        views.insert(key.to_owned(), v.clone());
        v
    }

    /// The shared `Entity<Toaster>` for a `ToastViewport` key.
    fn toaster(&self, key: &str, cx: &mut Context<ForgeView>) -> Entity<Toaster> {
        let mut toasters = self.toasters.borrow_mut();
        if let Some(t) = toasters.get(key) {
            return t.clone();
        }
        let t = cx.new(|_| Toaster::default());
        toasters.insert(key.to_owned(), t.clone());
        t
    }

    /// The named (or per-key) scroll handle shared between a `ScrollArea` and
    /// its scroll aids.
    fn handle(&self, name: &str) -> ScrollHandle {
        self.handles
            .borrow_mut()
            .entry(name.to_owned())
            .or_insert_with(ScrollHandle::new)
            .clone()
    }

    /// Render every child of `node` as proxies, keyed `{key}/{i}`.
    fn child_views(&self, node: &Node, key: &str, cx: &mut Context<ForgeView>) -> Vec<AnyView> {
        node.children
            .iter()
            .enumerate()
            .map(|(i, c)| self.child_view(c, &format!("{key}/{i}"), cx))
            .collect()
    }
}

/// `on <event>:` calls on a library node, as `Fn(&mut Window, &mut App)` —
/// most ely handlers take that exact shape. (Plain `on click:` is wrapped by
/// the runtime itself; this covers `change`, `confirm`, `close`, ….)
fn handler(
    view: &ForgeView,
    node: &Node,
    event: &str,
    cx: &mut Context<ForgeView>,
) -> Option<impl Fn(&mut Window, &mut App) + 'static> {
    let calls: Vec<(String, Vec<String>)> = node
        .events
        .iter()
        .filter(|e| e.event == event)
        .map(|e| {
            (
                e.handler.name.clone(),
                e.handler
                    .args
                    .iter()
                    .map(|a| {
                        view.resolve(a)
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_default()
                    })
                    .collect(),
            )
        })
        .collect();
    if calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(move |_: &mut Window, cx: &mut App| {
        for (name, args) in &calls {
            let weak = weak.clone();
            let name = name.clone();
            let args = args.clone();
            weak.update(cx, |view, cx| view.fire_call(&name, args, cx))
                .ok();
        }
    })
}

/// The state path a `{{…}}`-bound prop points at.
fn binding_of(node: &Node, name: &str) -> Option<Path> {
    match prop(node, name) {
        Some(Value::Binding { path, .. }) => Some(path.clone()),
        _ => None,
    }
}

/// Update the entity through a weak handle without panicking if a render pass
/// holds a borrow — mirrors the base's `try_update`.
fn try_update(
    weak: &WeakEntity<ForgeView>,
    cx: &mut App,
    f: impl FnOnce(&mut ForgeView, &mut Context<ForgeView>),
) {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let weak = weak.clone();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        weak.update(cx, |v, cx| f(v, cx)).ok();
    }));
}

/// `on <event>:` calls on a node, resolved to `(handler, args)` pairs.
fn calls_for(view: &ForgeView, node: &Node, event: &str) -> Vec<(String, Vec<String>)> {
    node.events
        .iter()
        .filter(|e| e.event == event)
        .map(|e| {
            (
                e.handler.name.clone(),
                e.handler
                    .args
                    .iter()
                    .map(|a| {
                        view.resolve(a)
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_default()
                    })
                    .collect(),
            )
        })
        .collect()
}

/// A `{{binding}}` on `value` plus `on change:` calls → callback taking the
/// new `Json` value (for selection/toggle/entry controls).
fn change(
    view: &ForgeView,
    node: &Node,
    cx: &mut Context<ForgeView>,
) -> Option<Rc<dyn Fn(Json, &mut Window, &mut App)>> {
    let path = binding_of(node, "value");
    let calls = calls_for(view, node, "change");
    if path.is_none() && calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(Rc::new(move |v: Json, _w: &mut Window, cx: &mut App| {
        if let Some(path) = path.clone() {
            let v = v.clone();
            try_update(&weak, cx, |view, cx| view.update_state(&path, v, cx));
        }
        for (name, args) in &calls {
            let (name, args) = (name.clone(), args.clone());
            try_update(&weak, cx, |view, cx| view.fire_call(&name, args, cx));
        }
    }))
}

/// `on <event>:` calls with one runtime string argument appended — for
/// "which item was picked" callbacks (menus, sheets, toggle groups).
fn change_str(
    view: &ForgeView,
    node: &Node,
    event: &str,
    cx: &mut Context<ForgeView>,
) -> Option<impl Fn(String, &mut Window, &mut App) + 'static> {
    let path = binding_of(node, "value");
    let calls = calls_for(view, node, event);
    if path.is_none() && calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(move |v: String, _w: &mut Window, cx: &mut App| {
        if let Some(path) = path.clone() {
            let v = Json::String(v.clone());
            try_update(&weak, cx, |view, cx| view.update_state(&path, v, cx));
        }
        for (name, args) in &calls {
            let (name, mut args) = (name.clone(), args.clone());
            args.push(v.clone());
            try_update(&weak, cx, |view, cx| view.fire_call(&name, args, cx));
        }
    })
}

/// Lucide name → `IconName`, searched across the bundled set.
/// Fire `on <event>:` calls with a string payload appended to each call's
/// args — like `change_str` but without touching the `value` binding.
fn fire_str(
    view: &ForgeView,
    node: &Node,
    event: &str,
    cx: &mut Context<ForgeView>,
) -> Option<impl Fn(String, &mut Window, &mut App) + 'static> {
    let calls = calls_for(view, node, event);
    if calls.is_empty() {
        return None;
    }
    let weak = cx.entity().downgrade();
    Some(move |v: String, _w: &mut Window, cx: &mut App| {
        for (name, args) in &calls {
            let (name, mut args) = (name.clone(), args.clone());
            args.push(v.clone());
            try_update(&weak, cx, |view, cx| view.fire_call(&name, args, cx));
        }
    })
}

/// Build a `TreeNode` forest from `/`-separated paths (`src/lib.rs` …).
fn tree_nodes(paths: &[String]) -> Vec<TreeNode> {
    fn into_nodes<'a>(it: impl Iterator<Item = &'a [String]>) -> Vec<TreeNode> {
        let mut nodes: Vec<TreeNode> = Vec::new();
        let mut grouped: std::collections::BTreeMap<&str, Vec<&[String]>> =
            std::collections::BTreeMap::new();
        for parts in it {
            match parts.split_first() {
                Some((head, rest)) if rest.is_empty() => {
                    nodes.push(TreeNode::new(head.trim(), head.trim()));
                }
                Some((head, rest)) => grouped.entry(head.trim()).or_default().push(rest),
                None => {}
            }
        }
        for (key, rests) in grouped {
            nodes.push(TreeNode::new(key, key).children(into_nodes(rests.into_iter())));
        }
        nodes
    }
    let parts: Vec<Vec<String>> = paths
        .iter()
        .map(|p| {
            p.split('/')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .collect();
    into_nodes(parts.iter().map(Vec::as_slice))
}

/// Build a `Menu` from an `items` list: `label`, `label|icon`, `label|icon|keys`,
/// `check:label`, `radio:label`, `@heading`, `-` separator. Each row fires an
/// `item:<label>` event when picked.
fn menu_of(node: &Node, view: &ForgeView, cx: &mut Context<ForgeView>) -> Menu {
    let mut menu = Menu::new();
    for entry in prop_list(node, "items") {
        let e = entry.trim();
        if e == "-" || e == "---" {
            menu = menu.separator();
            continue;
        }
        if let Some(title) = e.strip_prefix('@') {
            menu = menu.group(title.trim(), Vec::<MenuItem>::new());
            continue;
        }
        let (kind, rest) = match e.split_once(':') {
            Some(("check", rest)) => (Some(Kind::Check), rest),
            Some(("radio", rest)) => (Some(Kind::Radio), rest),
            _ => (None, e),
        };
        let mut p = rest.splitn(3, '|');
        let label = p.next().unwrap_or("").trim().to_owned();
        let mut item = match kind {
            Some(Kind::Check) => MenuItem::check(label.clone(), true),
            Some(Kind::Radio) => MenuItem::radio(label.clone(), true),
            _ => MenuItem::new(label.clone()),
        };
        if let Some(icon) = p.next().and_then(|n| icon_named(n.trim())) {
            item = item.icon(icon);
        }
        if let Some(keys) = p.next() {
            let keys = keys.trim();
            if !keys.is_empty() {
                item = item.keys(keys);
            }
        }
        if let Some(run) = fire_str(view, node, &format!("item:{label}"), cx) {
            item = item.on_click(move |w, cx| run(String::new(), w, cx));
        }
        menu = menu.item(item);
    }
    menu
}

enum Kind {
    Check,
    Radio,
}

fn platform_of(node: &Node) -> Option<Platform> {
    match prop_str(node, "platform").as_str() {
        "Mac" | "macOS" => Some(Platform::Mac),
        "Windows" => Some(Platform::Windows),
        "Linux" => Some(Platform::Linux),
        _ => None,
    }
}

fn update_state_of(node: &Node) -> UpdateState {
    match prop_str(node, "state").as_str() {
        s if s.starts_with("Downloading:") => {
            UpdateState::Downloading(s.trim_start_matches("Downloading:").parse().unwrap_or(0.0))
        }
        "Ready" => UpdateState::Ready,
        _ => UpdateState::Available,
    }
}

/// One table cell: bare text, or `n:`, `tag:text|Tone`, `progress:`, `spark:`,
/// `person:`, `link:label|url`, `dur:`, `empty`.
fn cell_of(raw: &str) -> TableCell {
    let (kind, rest) = match raw.split_once(':') {
        Some((k, r))
            if matches!(
                k,
                "n" | "tag" | "progress" | "spark" | "person" | "link" | "dur" | "empty"
            ) =>
        {
            (k, r)
        }
        _ => return TableCell::Text(raw.trim().into()),
    };
    match kind {
        "n" => TableCell::Number(rest.trim().parse().unwrap_or(0.0)),
        "tag" => {
            let mut p = rest.splitn(2, '|');
            let text = p.next().unwrap_or("").trim();
            let tone = match p.next().map(str::trim) {
                Some("Accent") => Tone::Accent,
                Some("Info") => Tone::Info,
                Some("Success") => Tone::Success,
                Some("Warning") => Tone::Warning,
                Some("Danger") => Tone::Danger,
                _ => Tone::Neutral,
            };
            TableCell::Tag(text.into(), tone)
        }
        "progress" => TableCell::Progress(rest.trim().parse().unwrap_or(0.0)),
        "spark" => TableCell::Spark(
            rest.split(',')
                .filter_map(|v| v.trim().parse().ok())
                .collect(),
        ),
        "person" => TableCell::Person(rest.trim().into()),
        "link" => {
            let mut p = rest.splitn(2, '|');
            TableCell::Link(
                p.next().unwrap_or("").trim().into(),
                p.next().unwrap_or("").trim().into(),
            )
        }
        "dur" => TableCell::Duration(rest.trim().parse().unwrap_or(0)),
        _ => TableCell::Empty,
    }
}

/// One table column: `key|title|flag,flag` — flags: end, unsorted, editable,
/// pinned, scale, agg:Sum, pre:X, suf:X, w:rems, dec:N.
fn column_of(raw: &str) -> Column {
    let mut p = raw.splitn(3, '|');
    let (key, title) = (p.next().unwrap_or("").trim(), p.next().unwrap_or("").trim());
    let title = if title.is_empty() { key } else { title };
    let mut col = Column::new(key, title);
    for flag in p.next().unwrap_or("").split(',').map(str::trim) {
        match flag {
            "end" => col = col.end(),
            "unsorted" => col = col.unsorted(),
            "editable" => col = col.editable(),
            "pinned" => col = col.pinned(),
            "scale" => col = col.scale(),
            f if f.starts_with("agg:") => {
                col = col.aggregate(match &f[4..] {
                    "Average" => Aggregate::Average,
                    "Count" => Aggregate::Count,
                    "Min" => Aggregate::Min,
                    "Max" => Aggregate::Max,
                    _ => Aggregate::Sum,
                })
            }
            f if f.starts_with("pre:") => col = col.prefix(&f[4..]),
            f if f.starts_with("suf:") => col = col.suffix(&f[4..]),
            f if f.starts_with("w:") => col = col.width(rems(f[2..].parse::<f32>().unwrap_or(0.0))),
            f if f.starts_with("dec:") => col = col.decimals(f[4..].parse().unwrap_or(0)),
            _ => {}
        }
    }
    col
}

fn density_of(raw: &str) -> Density {
    match raw {
        "Compact" => Density::Compact,
        "Comfortable" => Density::Comfortable,
        _ => Density::Standard,
    }
}

fn icon_named(name: &str) -> Option<IconName> {
    IconName::ALL.iter().copied().find(|i| i.name() == name)
}

fn icon_of(node: &Node) -> IconName {
    prop_str_opt(node, "icon")
        .and_then(|n| icon_named(&n))
        .unwrap_or(IconName::Plus)
}

impl Default for Ely {
    fn default() -> Self {
        Self::new()
    }
}

impl ForgeLibrary for Ely {
    fn alias(&self) -> &'static str {
        "ely"
    }

    fn catalog(&self) -> Option<&Catalog> {
        static CAT: OnceLock<Catalog> = OnceLock::new();
        Some(CAT.get_or_init(|| {
            catalog::from_yaml(include_str!("../../../catalogs/ely.catalog.yaml"))
                .expect("ely catalog parses")
        }))
    }

    fn render(
        &self,
        node: &Node,
        key: &str,
        view: &ForgeView,
        _window: &mut Window,
        cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement> {
        if !self.inited.replace(true) {
            ely_core::theme::Theme::init(cx);
            // Ely's Theme defaults to light — match the host's mode.
            if gpui_forge_runtime::base::kit::component::theme::Theme::global(cx).is_dark() {
                ely_core::theme::Theme::set_mode(ely_core::theme::Mode::Dark, cx);
            }
        }
        match node.element.name.as_str() {
            "Button" => Some(button_of(node, key).into_any_element()),
            "Badge" => Some(Badge::new(prop_str(node, "label")).into_any_element()),
            "Alert" => {
                let severity = match prop_str_opt(node, "severity").as_deref() {
                    Some("Success") => Severity::Success,
                    Some("Warning") => Severity::Warning,
                    Some("Danger") => Severity::Danger,
                    _ => Severity::Info,
                };
                let mut a = Alert::new(key.to_string(), severity, prop_str(node, "title"));
                if let Some(body) = prop_str_opt(node, "body") {
                    a = a.body(body);
                }
                Some(a.into_any_element())
            }
            "StatusMessage" => {
                let mut m = StatusMessage::new(key.to_string(), prop_str(node, "text"));
                if prop_str_opt(node, "icon").is_some() {
                    m = m.icon(IconName::Check);
                }
                Some(m.into_any_element())
            }
            "Spinner" => {
                let style = match prop_str_opt(node, "spinner").as_deref() {
                    Some("Dots") => SpinnerStyle::Dots,
                    Some("Wave") => SpinnerStyle::Wave,
                    Some("Bars") => SpinnerStyle::Bars,
                    Some("Pulse") => SpinnerStyle::Pulse,
                    _ => SpinnerStyle::Ring,
                };
                let size = match prop_str_opt(node, "size").as_deref() {
                    Some("Xs") => IconSize::Xs,
                    Some("Sm") => IconSize::Sm,
                    Some("Lg") => IconSize::Lg,
                    Some("Xl") => IconSize::Xl,
                    Some("Xxl") => IconSize::Xxl,
                    _ => IconSize::Md,
                };
                Some(
                    Spinner::new(key.to_string())
                        .style(style)
                        .size(size)
                        .into_any_element(),
                )
            }
            "Heading" => {
                let level = match prop(node, "level") {
                    Some(Value::Number { value, .. }) => *value as u8,
                    _ => 1,
                };
                Some(Heading::new(level, prop_str(node, "text")).into_any_element())
            }
            "Avatar" => {
                Some(Avatar::new(key.to_string(), prop_str(node, "name")).into_any_element())
            }
            "Paragraph" => Some(Paragraph::new(prop_str(node, "text")).into_any_element()),
            "Title" => Some(Title::new(prop_str(node, "text")).into_any_element()),
            "Divider" => Some(
                if prop_str_opt(node, "orientation").as_deref() == Some("vertical") {
                    Divider::vertical().into_any_element()
                } else {
                    Divider::horizontal().into_any_element()
                },
            ),
            "CopyButton" => {
                Some(CopyButton::new(key.to_string(), prop_str(node, "label")).into_any_element())
            }
            "TrendIndicator" => Some(
                TrendIndicator::new(prop_num(node, "change").unwrap_or(0.0)).into_any_element(),
            ),
            "Statistic" => Some(
                Statistic::new(
                    prop_str_opt(node, "label").unwrap_or_default(),
                    prop_str_opt(node, "value").unwrap_or_default(),
                    prop_num(node, "change").unwrap_or(0.0),
                )
                .into_any_element(),
            ),
            "KpiCard" => Some(
                KpiCard::new(Statistic::new(
                    prop_str_opt(node, "label").unwrap_or_default(),
                    prop_str_opt(node, "value").unwrap_or_default(),
                    prop_num(node, "change").unwrap_or(0.0),
                ))
                .into_any_element(),
            ),
            "Meter" => {
                // `value` is a share 0..=1. A number past 1 reads as a
                // percent (68 → 0.68). ely asserts on shares outside 0..=1,
                // so clamp rather than panic in the showcase.
                let v = prop_num(node, "value").unwrap_or(0.0) as f32;
                let share = if v > 1.0 { v / 100.0 } else { v }.clamp(0.0, 1.0);
                Some(
                    Meter::new(
                        key.to_string(),
                        prop_str_opt(node, "label").unwrap_or_default(),
                        share,
                    )
                    .into_any_element(),
                )
            }
            "UsageBar" => {
                Some(UsageBar::new(prop_num(node, "total").unwrap_or(100.0)).into_any_element())
            }
            "QrCode" => QrCode::new(prop_str_opt(node, "data").unwrap_or_default())
                .ok()
                .map(|q| q.into_any_element()),
            "TimelineItem" => Some(TimelineItem::new(prop_str(node, "text")).into_any_element()),
            "Banner" => Some(
                Banner::new(key.to_string(), severity_of(node), prop_str(node, "text"))
                    .into_any_element(),
            ),
            "InlineMessage" => Some(
                InlineMessage::new(severity_of(node), prop_str(node, "text")).into_any_element(),
            ),
            "Icon" => Some(Icon::new(icon_of(node)).into_any_element()),
            "Subtitle" => Some(Subtitle::new(prop_str(node, "text")).into_any_element()),
            "Caption" => Some(Caption::new(prop_str(node, "text")).into_any_element()),
            "Label" => Some(Label::new(prop_str(node, "text")).into_any_element()),
            "Overline" => Some(Overline::new(prop_str(node, "text")).into_any_element()),
            "Container" => {
                let size = match prop_str_opt(node, "size").as_deref() {
                    Some("Xs") => ContainerSize::Sm,
                    Some("Sm") => ContainerSize::Sm,
                    Some("Lg") => ContainerSize::Lg,
                    Some("Xl") => ContainerSize::Lg,
                    _ => ContainerSize::Md,
                };
                Some(Container::new(size).into_any_element())
            }
            "AspectRatio" => Some(
                AspectRatio::new(prop_num(node, "ratio").unwrap_or(1.0) as f32).into_any_element(),
            ),
            "ZStack" => Some(ZStack::new().into_any_element()),
            "IconButton" => {
                let icon = match prop_str_opt(node, "icon").as_deref() {
                    Some("check") => IconName::Check,
                    Some("circle-check") => IconName::CircleCheck,
                    Some("file-plus") => IconName::FilePlus,
                    Some("folder-plus") => IconName::FolderPlus,
                    Some("menu") => IconName::Menu,
                    Some("list-checks") => IconName::ListChecks,
                    Some("square-check") => IconName::SquareCheck,
                    Some("check-check") => IconName::CheckCheck,
                    _ => IconName::Plus,
                };
                let mut b = IconButton::new(key.to_string(), icon);
                if let Some(t) = prop_str_opt(node, "tooltip") {
                    b = b.tooltip(t);
                }
                if prop_bool(node, "disabled") {
                    b = b.disabled(true);
                }
                Some(b.into_any_element())
            }
            "Callout" => {
                let mut c = Callout::new(severity_of(node));
                if let Some(t) = prop_str_opt(node, "title") {
                    c = c.title(t);
                }
                Some(c.into_any_element())
            }
            "EmptyState" => Some(
                EmptyState::new(key.to_string(), icon_of(node), prop_str(node, "title"))
                    .into_any_element(),
            ),
            "Rating" => {
                let max = prop_num(node, "max").unwrap_or(5.0).max(1.0) as u8;
                let value = (prop_num(node, "value").unwrap_or(3.0) as u8).min(max);
                Some(
                    Rating::new(key.to_string(), value)
                        .max(max)
                        .into_any_element(),
                )
            }
            "SegmentedControl" => Some(
                SegmentedControl::new(key.to_string(), prop_str(node, "selected"))
                    .into_any_element(),
            ),
            "Stepper" => Some(
                Stepper::new(key.to_string(), prop_num(node, "value").unwrap_or(0.0))
                    .into_any_element(),
            ),
            "Switch" => {
                Some(Switch::new(key.to_string(), prop_bool(node, "checked")).into_any_element())
            }
            "Tag" => Some(Tag::new(key.to_string(), prop_str(node, "label")).into_any_element()),
            "ToggleButton" => {
                let mut item = ToggleItem::new(prop_str(node, "value"))
                    .label(prop_str(node, "label"))
                    .icon(icon_of(node));
                if let Some(t) = prop_str_opt(node, "tooltip") {
                    item = item.tooltip(t);
                }
                let mut b = ToggleButton::new(
                    key.to_string(),
                    item,
                    prop_bool(node, "checked") || prop_bool(node, "on"),
                );
                if let Some(h) = change(view, node, cx) {
                    b = b.on_toggle(move |on, w, cx| h(Json::Bool(on), w, cx));
                }
                Some(b.into_any_element())
            }
            "Collapsible" => {
                Some(Collapsible::new(key.to_string(), prop_bool(node, "open")).into_any_element())
            }
            "Disclosure" => {
                Some(Disclosure::new(key.to_string(), prop_bool(node, "open")).into_any_element())
            }
            "Sidebar" => {
                Some(Sidebar::new(key.to_string(), prop_bool(node, "collapsed")).into_any_element())
            }
            "Radio" => {
                Some(Radio::new(key.to_string(), prop_bool(node, "checked")).into_any_element())
            }
            "RadioCard" => Some(
                RadioCard::new(
                    key.to_string(),
                    prop_bool(node, "checked"),
                    prop_str(node, "title"),
                )
                .into_any_element(),
            ),
            "Checkbox" => {
                Some(Checkbox::new(key.to_string(), prop_bool(node, "checked")).into_any_element())
            }
            "Slider" => Some(
                Slider::new(key.to_string(), prop_num(node, "value").unwrap_or(0.0))
                    .into_any_element(),
            ),
            "Knob" => Some(
                Knob::new(key.to_string(), prop_num(node, "value").unwrap_or(0.0))
                    .into_any_element(),
            ),
            "ProgressBar" => Some(
                ProgressBar::new(
                    key.to_string(),
                    prop_num(node, "value").unwrap_or(0.0) as f32,
                )
                .into_any_element(),
            ),
            "NumberInput" => Some(
                NumberInput::new(key.to_string(), prop_num(node, "value").unwrap_or(0.0))
                    .into_any_element(),
            ),
            "Pagination" => Some(
                Pagination::new(
                    key.to_string(),
                    prop_num(node, "page").unwrap_or(1.0) as usize,
                    prop_num(node, "pages").unwrap_or(1.0) as usize,
                )
                .into_any_element(),
            ),
            "PinInput" => Some(
                PinInput::new(
                    key.to_string(),
                    prop_num(node, "length").unwrap_or(4.0).max(1.0) as usize,
                )
                .into_any_element(),
            ),
            "Form" => Some(Form::new(key.to_string()).into_any_element()),
            "Skeleton" => Some(Skeleton::new(key.to_string()).into_any_element()),
            "Reorder" => Some(Reorder::new(key.to_string()).into_any_element()),
            "Panel" => Some(Panel::new(prop_str(node, "title")).into_any_element()),
            "Fieldset" => Some(Fieldset::new(prop_str(node, "title")).into_any_element()),
            "Section" => Some(Section::new(prop_str(node, "title")).into_any_element()),
            "Code" => Some(Code::new(prop_str(node, "text")).into_any_element()),
            "CopyableText" => {
                Some(CopyableText::new(key.to_string(), prop_str(node, "text")).into_any_element())
            }
            "PercentText" => {
                Some(PercentText::new(prop_num(node, "value").unwrap_or(0.0)).into_any_element())
            }
            "NumberText" => {
                Some(NumberText::new(prop_num(node, "value").unwrap_or(0.0)).into_any_element())
            }
            "InlineEdit" => {
                Some(InlineEdit::new(key.to_string(), prop_str(node, "value")).into_any_element())
            }
            "Notification" => Some(
                Notification::new(key.to_string(), icon_of(node), prop_str(node, "title"))
                    .into_any_element(),
            ),
            "HotkeyInput" => Some(HotkeyInput::new(key.to_string(), None).into_any_element()),
            "Select" => Some(Select::new(key.to_string(), choices(node)).into_any_element()),
            "MultiSelect" => {
                Some(MultiSelect::new(key.to_string(), choices(node)).into_any_element())
            }
            "ChoiceChips" => {
                Some(ChoiceChips::new(key.to_string(), choices(node)).into_any_element())
            }
            "AvatarGroup" => {
                let mut g = AvatarGroup::new(
                    prop_list(node, "names")
                        .into_iter()
                        .enumerate()
                        .map(|(i, n)| Avatar::new(format!("{key}/{i}"), n)),
                )
                .size(avatar_size(node));
                if let Some(m) = prop_num(node, "max") {
                    g = g.max(m as usize);
                }
                Some(g.into_any_element())
            }
            "UserChip" => {
                let mut c = UserChip::new(key.to_string(), prop_str(node, "name"));
                if let Some(src) = prop_str_opt(node, "image") {
                    c = c.image(std::path::PathBuf::from(src));
                }
                Some(c.into_any_element())
            }
            "CountBadge" => {
                let mut b = CountBadge::new(
                    key.to_string(),
                    prop_num(node, "count").unwrap_or(0.0) as usize,
                );
                if let Some(m) = prop_num(node, "max") {
                    b = b.max(m as usize);
                }
                if let Some(c) = node.children.first() {
                    b = b.over(self.child_view(c, &format!("{key}/0"), cx));
                }
                Some(b.into_any_element())
            }
            "DotBadge" => {
                let mut b = DotBadge::new().tone(tone_of(node));
                if let Some(c) = node.children.first() {
                    b = b.over(self.child_view(c, &format!("{key}/0"), cx));
                }
                Some(b.into_any_element())
            }
            "BeforeAfter" => {
                let mut it = node.children.iter();
                let (Some(before), Some(after)) = (it.next(), it.next()) else {
                    return None;
                };
                Some(
                    BeforeAfter::new(
                        key.to_string(),
                        self.child_view(before, &format!("{key}/0"), cx),
                        self.child_view(after, &format!("{key}/1"), cx),
                    )
                    .into_any_element(),
                )
            }
            "Changelog" => {
                let mut c = Changelog::new();
                for child in node.children.iter() {
                    let mut r = Release::new(prop_str(child, "version"), prop_str(child, "date"));
                    if let Some(s) = prop_str_opt(child, "summary") {
                        r = r.summary(s);
                    }
                    for entry in prop_str(child, "changes").split(';') {
                        let mut parts = entry.splitn(2, ':');
                        let (Some(kind), Some(text)) = (parts.next(), parts.next()) else {
                            continue;
                        };
                        let kind = match kind.trim() {
                            "changed" => Change::Changed,
                            "deprecated" => Change::Deprecated,
                            "removed" => Change::Removed,
                            "fixed" => Change::Fixed,
                            "security" => Change::Security,
                            _ => Change::Added,
                        };
                        r = r.change(kind, text.trim().to_owned());
                    }
                    c = c.release(r);
                }
                Some(c.into_any_element())
            }
            "Comparison" => {
                let mut c = Comparison::new(prop_list(node, "columns"));
                for entry in prop_list(node, "rows") {
                    let mut parts = entry.split('|');
                    let (Some(name), cells) = (parts.next(), parts.collect::<Vec<_>>()) else {
                        continue;
                    };
                    if cells
                        .iter()
                        .all(|v| matches!(*v, "yes" | "no" | "true" | "false"))
                    {
                        c = c.marks(
                            name.trim(),
                            cells.iter().map(|v| matches!(*v, "yes" | "true")),
                        );
                    } else {
                        c = c.row(name.trim(), cells.iter().map(|v| v.trim()));
                    }
                }
                Some(c.into_any_element())
            }
            "ActivityFeed" => {
                let mut f = ActivityFeed::new(key.to_string());
                for child in &node.children {
                    let at: Timestamp = prop_str(child, "at")
                        .parse()
                        .unwrap_or_else(|_| Timestamp::now());
                    let mut a = Activity::new(
                        key.to_string(),
                        prop_str(child, "actor"),
                        prop_str(child, "verb"),
                        at,
                    );
                    if let Some(o) = prop_str_opt(child, "object") {
                        a = a.object(o);
                    }
                    if let Some(i) = prop_str_opt(child, "icon").and_then(|n| icon_named(&n)) {
                        a = a.icon(i);
                    }
                    f = f.entry(a);
                }
                Some(f.into_any_element())
            }
            "Gauge" => Some(
                Gauge::new(
                    key.to_string(),
                    prop_str(node, "label"),
                    prop_num(node, "value").unwrap_or(0.0),
                    prop_num(node, "min").unwrap_or(0.0),
                    prop_num(node, "max").unwrap_or(100.0).max(1.0),
                )
                .into_any_element(),
            ),
            "DescriptionList" => {
                let mut d = DescriptionList::new();
                for entry in prop_list(node, "rows") {
                    let mut parts = entry.splitn(2, '|');
                    if let (Some(l), Some(t)) = (parts.next(), parts.next()) {
                        d = d.text(l.trim(), t.trim());
                    }
                }
                Some(d.into_any_element())
            }
            "PropertyGrid" => {
                let mut g = PropertyGrid::new(key.to_string());
                for child in &node.children {
                    let mut pg = PropertyGroup::new(prop_str(child, "title"));
                    for entry in prop_list(child, "rows") {
                        let mut parts = entry.splitn(2, '|');
                        if let (Some(n), Some(v)) = (parts.next(), parts.next()) {
                            pg = pg.row(n.trim(), Paragraph::new(v.trim().to_owned()));
                        }
                    }
                    g = g.group(pg);
                }
                Some(g.into_any_element())
            }
            "Sparkline" => {
                let mut s = Sparkline::new(prop_floats(node, "values"));
                s = match prop_str(node, "kind").as_str() {
                    "area" => s.area(),
                    "bars" => s.bars(),
                    _ => s,
                };
                Some(s.into_any_element())
            }
            "Stars" => {
                Some(Stars::new(prop_num(node, "score").unwrap_or(0.0) as f32).into_any_element())
            }
            "Timeline" => {
                let mut t = Timeline::new();
                for (i, child) in node.children.iter().enumerate() {
                    let mut item = TimelineItem::new(Paragraph::new(prop_str(child, "title")));
                    if let Some(time) = prop_str_opt(child, "time") {
                        item = item.time(Paragraph::new(time));
                    }
                    let k = format!("{key}/{i}");
                    item.extend([self.child_view(child, &k, cx).into_any_element()]);
                    t = t.item(item);
                }
                Some(t.into_any_element())
            }
            "Watermark" => {
                Some(Watermark::new(key.to_string(), prop_str(node, "text")).into_any_element())
            }
            "Gallery" => {
                let slides = prop_list(node, "slides")
                    .into_iter()
                    .filter_map(|entry| {
                        let mut parts = entry.splitn(2, '|');
                        Some(Slide::new(
                            std::path::PathBuf::from(parts.next()?.trim()),
                            parts.next().unwrap_or("").trim().to_owned(),
                        ))
                    })
                    .collect::<Vec<_>>();
                let mut g = Gallery::new(key.to_string(), slides);
                if let Some(c) = prop_num(node, "columns") {
                    g = g.columns(c as u16);
                }
                Some(g.into_any_element())
            }
            "Carousel" => {
                let mut c = Carousel::new(key.to_string());
                c.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(c.into_any_element())
            }
            "Barcode" => match prop_str(node, "kind").as_str() {
                "ean13" => Barcode::ean13(&prop_str(node, "data")),
                _ => Barcode::code128(&prop_str(node, "data")),
            }
            .ok()
            .map(|b| b.into_any_element()),
            "ActionBar" => {
                let mut bar = ActionBar::new();
                if prop_bool(node, "floating") {
                    bar = bar.floating();
                }
                bar.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(bar.into_any_element())
            }
            "BulkActionBar" => {
                let mut bar = BulkActionBar::new(
                    key.to_string(),
                    prop_num(node, "count").unwrap_or(0.0) as usize,
                );
                for el in self
                    .child_views(node, key, cx)
                    .into_iter()
                    .map(|v| v.into_any_element())
                {
                    bar = bar.action(el);
                }
                if let Some(h) = handler(view, node, "clear", cx) {
                    bar = bar.on_clear(h);
                }
                Some(bar.into_any_element())
            }
            "ConfirmButton" => {
                let mut b = ConfirmButton::new(
                    key.to_string(),
                    prop_str(node, "label"),
                    match prop_str(node, "mode").as_str() {
                        "hold" | "Hold" => ConfirmMode::Hold,
                        _ => ConfirmMode::Twice,
                    },
                );
                if let Some(l) = prop_str_opt(node, "armed") {
                    b = b.armed_label(l);
                }
                if let Some(h) = handler(view, node, "confirm", cx) {
                    b = b.on_confirm(h);
                }
                Some(b.into_any_element())
            }
            "FloatingActionButton" => {
                let mut b = FloatingActionButton::new(key.to_string(), icon_of(node));
                if let Some(l) = prop_str_opt(node, "label") {
                    b = b.label(l);
                }
                if let Some(t) = prop_str_opt(node, "tooltip") {
                    b = b.tooltip(t);
                }
                Some(b.into_any_element())
            }
            "ButtonGroup" => {
                let mut g = ButtonGroup::new();
                for (i, child) in node.children.iter().enumerate() {
                    g = g.button(button_of(child, &format!("{key}/{i}")));
                }
                Some(g.into_any_element())
            }
            "ToggleGroup" => {
                let mut g = ToggleGroup::new(key.to_string());
                for entry in prop_list(node, "items") {
                    let mut parts = entry.splitn(3, '|');
                    let Some(value) = parts.next() else {
                        continue;
                    };
                    let label = parts
                        .clone()
                        .next()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .unwrap_or(value.trim());
                    let mut item = ToggleItem::new(value.trim()).label(label.to_owned());
                    if let Some(icon) = parts.nth(1).and_then(|n| icon_named(n.trim())) {
                        item = item.icon(icon);
                    }
                    g = g.item(item);
                }
                if prop_bool(node, "multiple") {
                    g = g.multiple();
                }
                match prop(node, "value") {
                    Some(Value::List(items)) => {
                        g = g.selected(items.iter().filter_map(|v| match v {
                            Value::String(s) | Value::Enum(s) => Some(s.clone()),
                            _ => None,
                        }));
                    }
                    _ => {
                        let sel = prop_str(node, "value");
                        if !sel.is_empty() {
                            g = g.selected([sel]);
                        }
                    }
                }
                if let Some(h) = change_str(view, node, "change", cx) {
                    g = g.on_change(move |values, w, cx| h(values.join(","), w, cx));
                }
                Some(g.into_any_element())
            }
            "QuickActions" => {
                let mut q = QuickActions::new(key.to_string());
                for entry in prop_list(node, "actions") {
                    let mut parts = entry.splitn(3, '|');
                    let (Some(icon), Some(label)) = (parts.next(), parts.next()) else {
                        continue;
                    };
                    let pick = change_str(view, node, "pick", cx);
                    let l = label.trim().to_owned();
                    let mut a = QuickAction::new(
                        icon_named(icon.trim()).unwrap_or(IconName::Zap),
                        l.clone(),
                        move |w, cx| {
                            if let Some(pick) = &pick {
                                pick(l.clone(), w, cx);
                            }
                        },
                    );
                    if let Some(note) = parts.next() {
                        a = a.note(note.trim());
                    }
                    q = q.action(a);
                }
                Some(q.into_any_element())
            }
            "ShareButton" => {
                let text = prop_str_opt(node, "text");
                let url = prop_str_opt(node, "url");
                if text.is_none() && url.is_none() {
                    return None;
                }
                let mut b = ShareButton::new(key.to_string());
                if let Some(t) = text {
                    b = b.text(t);
                }
                if let Some(u) = url {
                    b = b.url(u);
                }
                Some(b.into_any_element())
            }
            "ActionSheet" => {
                let close = handler(view, node, "close", cx);
                let mut s = ActionSheet::new(key.to_string(), move |w, cx| {
                    if let Some(h) = &close {
                        h(w, cx);
                    }
                });
                if let Some(t) = prop_str_opt(node, "title") {
                    s = s.title(t);
                }
                if let Some(m) = prop_str_opt(node, "message") {
                    s = s.message(m);
                }
                for entry in prop_list(node, "actions") {
                    let pick = change_str(view, node, "pick", cx);
                    let (label, danger) = match entry.strip_prefix("danger:") {
                        Some(l) => (l.trim().to_owned(), true),
                        None => (entry.trim().to_owned(), false),
                    };
                    let l2 = label.clone();
                    let run = move |w: &mut Window, cx: &mut App| {
                        if let Some(p) = &pick {
                            p(l2.clone(), w, cx);
                        }
                    };
                    s = if danger {
                        s.destructive(label, run)
                    } else {
                        s.action(label, run)
                    };
                }
                Some(s.into_any_element())
            }
            "Countdown" => {
                let secs = prop_num(node, "seconds").unwrap_or(60.0);
                let mut c = Countdown::new(
                    key.to_string(),
                    Instant::now() + Duration::from_secs_f64(secs.max(0.0)),
                )
                .size(text_size(node));
                if let Some(h) = handler(view, node, "done", cx) {
                    c = c.on_done(h);
                }
                Some(c.into_any_element())
            }
            "Timer" => {
                let secs = prop_num(node, "seconds").unwrap_or(0.0);
                Some(
                    Timer::new(
                        key.to_string(),
                        Instant::now() - Duration::from_secs_f64(secs.max(0.0)),
                    )
                    .size(text_size(node))
                    .into_any_element(),
                )
            }
            "ConfirmationCard" => {
                let mut c = ConfirmationCard::new(key.to_string(), prop_str(node, "title"));
                if let Some(b) = prop_str_opt(node, "body") {
                    c = c.body(b);
                }
                if let Some(l) = prop_str_opt(node, "confirm_label") {
                    c = c.confirm(l);
                }
                if prop_bool(node, "destructive") {
                    c = c.destructive();
                }
                if let Some(h) = handler(view, node, "confirm", cx) {
                    c = c.on_confirm(h);
                }
                if let Some(h) = handler(view, node, "cancel", cx) {
                    c = c.on_cancel(h);
                }
                Some(c.into_any_element())
            }
            "NotificationCenter" => {
                let mut center = NotificationCenter::new(key.to_string());
                let mut groups: Vec<(String, Vec<Notification>)> = Vec::new();
                for (i, child) in node.children.iter().enumerate() {
                    let group = prop_str(child, "group");
                    let group = if group.is_empty() { "New" } else { &group };
                    let n = notification_of(child, &format!("{key}/{i}"));
                    if let Some((_, items)) = groups.iter_mut().find(|(h, _)| h == group) {
                        items.push(n);
                    } else {
                        groups.push((group.to_owned(), vec![n]));
                    }
                }
                for (heading, items) in groups {
                    center = center.group(heading, items);
                }
                if let Some(h) = handler(view, node, "read_all", cx) {
                    center = center.on_read_all(h);
                }
                if let Some(h) = handler(view, node, "clear", cx) {
                    center = center.on_clear(h);
                }
                Some(center.into_any_element())
            }
            "ResultView" => {
                let mut r = match prop_str(node, "status").as_str() {
                    "failure" | "Failure" => {
                        ResultView::failure(key.to_string(), prop_str(node, "title"))
                    }
                    _ => ResultView::success(key.to_string(), prop_str(node, "title")),
                };
                if let Some(b) = prop_str_opt(node, "body") {
                    r = r.body(b);
                }
                for el in self
                    .child_views(node, key, cx)
                    .into_iter()
                    .map(|v| v.into_any_element())
                {
                    r = r.action(el);
                }
                Some(r.into_any_element())
            }
            "ErrorView" => {
                let mut e = match prop_str(node, "kind").as_str() {
                    "not_found" => ErrorView::not_found(key.to_string()),
                    "no_permission" => ErrorView::no_permission(key.to_string()),
                    "maintenance" => ErrorView::maintenance(key.to_string()),
                    "offline" => ErrorView::offline(key.to_string()),
                    "code" => ErrorView::with_code(
                        key.to_string(),
                        prop_str(node, "code"),
                        prop_str(node, "title"),
                    ),
                    _ => ErrorView::new(key.to_string(), icon_of(node), prop_str(node, "title")),
                };
                if let Some(d) = prop_str_opt(node, "detail") {
                    e = e.detail(d);
                }
                Some(e.into_any_element())
            }
            "ErrorBoundary" => {
                let mut b = ErrorBoundary::new(
                    key.to_string(),
                    node.children.first().map_or_else(
                        || Err(anyhow::anyhow!("empty boundary")),
                        |c| {
                            Ok(self
                                .child_view(c, &format!("{key}/0"), cx)
                                .into_any_element())
                        },
                    ),
                );
                if let Some(h) = handler(view, node, "retry", cx) {
                    b = b.on_retry(h);
                }
                Some(b.into_any_element())
            }
            "Suspense" => {
                let child = node.children.first().map(|c| {
                    Ok(self
                        .child_view(c, &format!("{key}/0"), cx)
                        .into_any_element())
                });
                let content = match prop_str(node, "state").as_str() {
                    "error" => Some(Err(anyhow::anyhow!(prop_str(node, "error")))),
                    "waiting" => None,
                    _ => child,
                };
                let mut s = Suspense::new(key.to_string(), content);
                if let Some(f) = node.children.get(1) {
                    s = s.fallback(self.child_view(f, &format!("{key}/1"), cx));
                } else if let Some(f) = prop_str_opt(node, "fallback") {
                    s = s.fallback(Paragraph::new(f));
                }
                if let Some(h) = handler(view, node, "retry", cx) {
                    s = s.on_retry(h);
                }
                Some(s.into_any_element())
            }
            "SavingIndicator" => {
                let mut s = SavingIndicator::new(
                    key.to_string(),
                    match prop_str(node, "state").as_str() {
                        "Saved" | "saved" => SaveState::Saved,
                        "Failed" | "failed" => SaveState::Failed,
                        _ => SaveState::Saving,
                    },
                );
                if let Some(h) = handler(view, node, "retry", cx) {
                    s = s.on_retry(h);
                }
                Some(s.into_any_element())
            }
            "SyncStatus" => {
                let state = match prop_str(node, "state").as_str() {
                    "Syncing" | "syncing" => {
                        SyncState::Syncing(prop_num(node, "left").unwrap_or(0.0) as usize)
                    }
                    "Paused" | "paused" => SyncState::Paused,
                    "Failed" | "failed" => SyncState::Failed,
                    _ => SyncState::Synced,
                };
                Some(SyncStatus::new(key.to_string(), state).into_any_element())
            }
            "ConnectionStatus" => {
                let state = match prop_str(node, "state").as_str() {
                    "Offline" | "offline" => Connectivity::Offline,
                    "Reconnecting" | "reconnecting" => Connectivity::Reconnecting,
                    _ => Connectivity::Online,
                };
                Some(ConnectionStatus::new(key.to_string(), state).into_any_element())
            }
            "ToastViewport" => {
                let fresh = !self.toasters.borrow().contains_key(key);
                let toaster = self.toaster(key, cx);
                if fresh {
                    // Seed once — pushing on every render would duplicate.
                    for entry in prop_list(node, "toasts") {
                        let mut parts = entry.splitn(3, '|');
                        let (severity, title, body) = (
                            parts.next().unwrap_or("info"),
                            parts.next().unwrap_or("").to_owned(),
                            parts.next().map(str::to_owned),
                        );
                        let mut t = Toast::new(title).severity(match severity.trim() {
                            "success" | "Success" => Severity::Success,
                            "warning" | "Warning" => Severity::Warning,
                            "danger" | "Danger" | "error" => Severity::Danger,
                            _ => Severity::Info,
                        });
                        if let Some(b) = body {
                            t = t.body(b);
                        }
                        toaster.update(cx, |t2, cx| {
                            t2.push(t, cx);
                        });
                    }
                }
                Some(ToastViewport::new(key.to_string(), &toaster).into_any_element())
            }
            "Accordion" => {
                let mut a = Accordion::new(key.to_string());
                if prop_bool(node, "multiple") {
                    a = a.multiple();
                }
                if prop_bool(node, "first_open") {
                    a = a.first_open();
                }
                for (i, child) in node.children.iter().enumerate() {
                    let mut item = AccordionItem::new(prop_str(child, "title"));
                    item.extend(
                        self.child_views(child, &format!("{key}/{i}"), cx)
                            .into_iter()
                            .map(|v| v.into_any_element()),
                    );
                    a = a.item(item);
                }
                Some(a.into_any_element())
            }
            "SimpleGrid" => {
                let mut g = SimpleGrid::new(
                    key.to_string(),
                    px(prop_num(node, "min").unwrap_or(160.0) as f32),
                    px(prop_num(node, "gap").unwrap_or(8.0) as f32),
                );
                g.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(g.into_any_element())
            }
            "Masonry" => {
                let mut m = Masonry::new(
                    key.to_string(),
                    prop_num(node, "columns").unwrap_or(2.0).max(1.0) as usize,
                    px(prop_num(node, "gap").unwrap_or(8.0) as f32),
                );
                m.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(m.into_any_element())
            }
            "Page" => {
                let mut p = Page::new(prop_str(node, "title"));
                if let Some(s) = prop_str_opt(node, "subtitle") {
                    p = p.subtitle(s);
                }
                let mut body = Vec::new();
                for (i, child) in node.children.iter().enumerate() {
                    let el = self
                        .child_view(child, &format!("{key}/{i}"), cx)
                        .into_any_element();
                    if prop_str(child, "slot") == "actions" {
                        p = p.action(el);
                    } else {
                        body.push(el);
                    }
                }
                p.extend(body);
                Some(p.into_any_element())
            }
            "AppShell" => {
                let mut shell = AppShell::new();
                let mut body = Vec::new();
                for (i, child) in node.children.iter().enumerate() {
                    let el = self
                        .child_view(child, &format!("{key}/{i}"), cx)
                        .into_any_element();
                    match prop_str(child, "slot").as_str() {
                        "title_bar" | "header" => shell = shell.title_bar(el),
                        "sidebar" => shell = shell.sidebar(el),
                        "status_bar" | "footer" => shell = shell.status_bar(el),
                        _ => body.push(el),
                    }
                }
                shell.extend(body);
                Some(shell.into_any_element())
            }
            "MasterDetail" => {
                let (Some(master), Some(detail)) = (node.children.first(), node.children.get(1))
                else {
                    return None;
                };
                Some(
                    MasterDetail::new(
                        key.to_string(),
                        self.child_view(master, &format!("{key}/0"), cx),
                        self.child_view(detail, &format!("{key}/1"), cx),
                        px(prop_num(node, "min").unwrap_or(160.0) as f32),
                    )
                    .into_any_element(),
                )
            }
            "PaneGroup" => {
                let fresh = !self.panes.borrow().contains_key(key);
                let group = {
                    let mut panes = self.panes.borrow_mut();
                    panes
                        .entry(key.to_owned())
                        .or_insert_with(|| {
                            let views = self.child_views(node, key, cx);
                            cx.new(|_| {
                                PaneGroup::new(move |id: PaneId, _, _| {
                                    views
                                        .get(id.saturating_sub(1) as usize)
                                        .or_else(|| views.last())
                                        .cloned()
                                        .map(|v| v.into_any_element())
                                        .unwrap_or_else(|| div().into_any_element())
                                })
                            })
                        })
                        .clone()
                };
                if fresh {
                    let axis = match prop_str(node, "axis").as_str() {
                        "vertical" | "Vertical" => Axis::Vertical,
                        _ => Axis::Horizontal,
                    };
                    let extra = (prop_num(node, "panes").unwrap_or(1.0) as u64).saturating_sub(1);
                    group.update(cx, |pg, cx| {
                        for _ in 0..extra {
                            let first = pg.panes().first().copied().unwrap_or(1);
                            pg.split(first, axis, cx);
                        }
                    });
                }
                Some(group.into_any_element())
            }
            "Dock" => {
                let fresh = !self.docks.borrow().contains_key(key);
                let dock = {
                    let mut docks = self.docks.borrow_mut();
                    docks
                        .entry(key.to_owned())
                        .or_insert_with(|| {
                            let center_view = node
                                .children
                                .first()
                                .map(|c| self.child_view(c, &format!("{key}/center"), cx));
                            let views = self.child_views(node, key, cx);
                            let panel_ids: Vec<String> =
                                node.children.iter().map(|c| prop_str(c, "panel")).collect();
                            cx.new(|cx| {
                                let center = move |_: &mut Window, _: &mut App| -> AnyElement {
                                    center_view
                                        .clone()
                                        .map(|v| v.into_any_element())
                                        .unwrap_or_else(|| div().into_any_element())
                                };
                                let views = views.clone();
                                let content = move |id: &SharedString,
                                                    _: &mut Window,
                                                    _: &mut App|
                                      -> AnyElement {
                                    views
                                        .iter()
                                        .enumerate()
                                        .find_map(|(i, v)| {
                                            (panel_ids.get(i).is_some_and(|p| p == id.as_str()))
                                                .then(|| v.clone().into_any_element())
                                        })
                                        .unwrap_or_else(|| div().into_any_element())
                                };
                                Dock::new(center, content, cx)
                            })
                        })
                        .clone()
                };
                if fresh {
                    for entry in prop_list(node, "panels") {
                        let mut parts = entry.splitn(4, '|');
                        let (Some(id), Some(title)) = (parts.next(), parts.next()) else {
                            continue;
                        };
                        let panel = DockPanel {
                            id: id.trim().into(),
                            title: title.trim().into(),
                            icon: parts
                                .next()
                                .and_then(|n| icon_named(n.trim()))
                                .unwrap_or(IconName::Square),
                        };
                        let side = match parts.next().map(str::trim) {
                            Some("right") => DockSide::Right,
                            Some("top") => DockSide::Top,
                            Some("bottom") => DockSide::Bottom,
                            _ => DockSide::Left,
                        };
                        dock.update(cx, |d, _| d.add(panel, side));
                    }
                }
                Some(dock.into_any_element())
            }
            "Sheet" => {
                let close = handler(view, node, "close", cx);
                let mut s = Sheet::new(
                    key.to_string(),
                    match prop_str(node, "side").as_str() {
                        "Left" | "left" => Place::Left,
                        "Top" | "top" => Place::Top,
                        "Bottom" | "bottom" => Place::Bottom,
                        _ => Place::Right,
                    },
                    prop_str(node, "title"),
                    move |w, cx| {
                        if let Some(h) = &close {
                            h(w, cx);
                        }
                    },
                );
                s.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(s.into_any_element())
            }
            "Drawer" => {
                let close = handler(view, node, "close", cx);
                let mut d = Drawer::new(key.to_string(), move |w, cx| {
                    if let Some(h) = &close {
                        h(w, cx);
                    }
                });
                d.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(d.into_any_element())
            }
            "FloatingPanel" => {
                let mut p = FloatingPanel::new(
                    key.to_string(),
                    prop_str(node, "title"),
                    point(
                        px(prop_num(node, "x").unwrap_or(80.0) as f32),
                        px(prop_num(node, "y").unwrap_or(80.0) as f32),
                    ),
                );
                if let Some(h) = change_str(view, node, "move", cx) {
                    p = p.on_move(move |pt, w, cx| {
                        h(format!("{},{}", f32::from(pt.x), f32::from(pt.y)), w, cx)
                    });
                }
                p.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(p.into_any_element())
            }
            "SplitPane" => {
                let mut s = SplitPane::new(
                    key.to_string(),
                    match prop_str(node, "axis").as_str() {
                        "vertical" | "Vertical" => Axis::Vertical,
                        _ => Axis::Horizontal,
                    },
                    px(prop_num(node, "min").unwrap_or(120.0) as f32),
                );
                for v in self.child_views(node, key, cx) {
                    s = s.pane(v);
                }
                let sizes = prop_floats(node, "sizes");
                if !sizes.is_empty() {
                    s = s.sizes(&sizes);
                }
                if let Some(h) = change_str(view, node, "resize", cx) {
                    s = s.on_resize(move |sizes, w, cx| {
                        h(
                            sizes
                                .iter()
                                .map(|f| f.to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                            w,
                            cx,
                        )
                    });
                }
                Some(s.into_any_element())
            }
            "ScrollArea" => {
                let mut a = ScrollArea::new(key.to_string());
                if prop_bool(node, "vertical") {
                    a = a.vertical();
                }
                if prop_bool(node, "shadows") {
                    a = a.shadows();
                }
                let name = prop_str(node, "handle");
                if !name.is_empty() {
                    a = a.track_scroll(&self.handle(&name));
                }
                a.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(a.into_any_element())
            }
            "ScrollShadow" => {
                let name = prop_str(node, "handle");
                let h = self.handle(if name.is_empty() { key } else { &name });
                Some(ScrollShadow::new(&h).into_any_element())
            }
            "ScrollToTop" => {
                let name = prop_str(node, "handle");
                let h = self.handle(if name.is_empty() { key } else { &name });
                Some(ScrollToTop::new(key.to_string(), &h).into_any_element())
            }
            "StickyHeader" => {
                let name = prop_str(node, "handle");
                let h = self.handle(if name.is_empty() { key } else { &name });
                let head = node
                    .children
                    .first()
                    .map(|c| self.child_view(c, &format!("{key}/head"), cx));
                let mut s = StickyHeader::new(key.to_string(), &h, move |_, _| {
                    head.clone()
                        .map(|v| v.into_any_element())
                        .unwrap_or_else(|| div().into_any_element())
                });
                for v in self.child_views(
                    &Node {
                        children: node.children.iter().skip(1).cloned().collect(),
                        ..node.clone()
                    },
                    &format!("{key}/body"),
                    cx,
                ) {
                    s.extend([v.into_any_element()]);
                }
                Some(s.into_any_element())
            }
            "Viewport" => {
                let content = node
                    .children
                    .first()
                    .map(|c| self.child_view(c, &format!("{key}/0"), cx));
                let mut v = Viewport::new(key.to_string(), move |t, _, _| {
                    let inner = content
                        .clone()
                        .map(|v| v.into_any_element())
                        .unwrap_or_else(|| div().into_any_element());
                    div()
                        .relative()
                        .size_full()
                        .child(
                            div()
                                .absolute()
                                .left(t.offset.x)
                                .top(t.offset.y)
                                .child(inner),
                        )
                        .into_any_element()
                });
                if let (Some(min), Some(max)) =
                    (prop_num(node, "min_zoom"), prop_num(node, "max_zoom"))
                {
                    v = v.zoom_range(min as f32, max as f32);
                }
                Some(v.into_any_element())
            }
            "Card" => {
                let mut c = Card::new();
                let mut body = Vec::new();
                for (i, child) in node.children.iter().enumerate() {
                    let k = format!("{key}/{i}");
                    match prop_str(child, "slot").as_str() {
                        "header" => {
                            let mut h = CardHeader::new(prop_str(child, "title"));
                            if let Some(d) = prop_str_opt(child, "description") {
                                h = h.description(d);
                            }
                            c = c.header(h);
                        }
                        "footer" => {
                            c = c.footer(self.child_view(child, &k, cx));
                        }
                        _ => body.push(self.child_view(child, &k, cx).into_any_element()),
                    }
                }
                c.extend(body);
                Some(c.into_any_element())
            }
            "CardHeader" => {
                let mut h = CardHeader::new(prop_str(node, "title"));
                if let Some(d) = prop_str_opt(node, "description") {
                    h = h.description(d);
                }
                if let Some(child) = node.children.first() {
                    h = h.action(self.child_view(child, &format!("{key}/0"), cx));
                }
                Some(h.into_any_element())
            }
            "Frame" => {
                let mut f = Frame::new();
                f.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(f.into_any_element())
            }
            "Well" => {
                let mut w = Well::new();
                w.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(w.into_any_element())
            }
            "Input" => {
                Some(Input::new(&self.field(node, key, view, _window, cx)).into_any_element())
            }
            "PasswordInput" => Some(
                PasswordInput::new(&self.field(node, key, view, _window, cx)).into_any_element(),
            ),
            "EmailInput" => {
                Some(EmailInput::new(&self.field(node, key, view, _window, cx)).into_any_element())
            }
            "UrlInput" => {
                Some(UrlInput::new(&self.field(node, key, view, _window, cx)).into_any_element())
            }
            "SearchInput" => {
                let mut s =
                    SearchInput::new(key.to_string(), &self.field(node, key, view, _window, cx));
                if let Some(h) = change_str(view, node, "pick", cx) {
                    s = s.on_pick(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(s.into_any_element())
            }
            "MaskedInput" => Some(
                MaskedInput::new(
                    key.to_string(),
                    &self.field(node, key, view, _window, cx),
                    prop_str(node, "mask"),
                )
                .into_any_element(),
            ),
            "PhoneInput" => {
                let want = prop_str(node, "country");
                let country = COUNTRIES
                    .iter()
                    .find(|c| c.0 == want.as_str())
                    .map(|c| c.0)
                    .unwrap_or("US");
                Some(
                    PhoneInput::new(
                        key.to_string(),
                        &self.field(node, key, view, _window, cx),
                        country,
                    )
                    .into_any_element(),
                )
            }
            "PathInput" => {
                let mut p =
                    PathInput::new(key.to_string(), &self.field(node, key, view, _window, cx));
                if let Some(h) = change_str(view, node, "choose", cx) {
                    p = p.on_choose(move |v, w, cx| h(v.display().to_string(), w, cx));
                }
                Some(p.into_any_element())
            }
            "RegexInput" => {
                Some(RegexInput::new(&self.field(node, key, view, _window, cx)).into_any_element())
            }
            "ExpressionInput" => Some(
                ExpressionInput::new(&self.field(node, key, view, _window, cx)).into_any_element(),
            ),
            "CodeInput" => {
                Some(CodeInput::new(&self.field(node, key, view, _window, cx)).into_any_element())
            }
            "JsonInput" => {
                Some(JsonInput::new(&self.field(node, key, view, _window, cx)).into_any_element())
            }
            "MentionInput" => Some(
                MentionInput::new(key.to_string(), &self.field(node, key, view, _window, cx))
                    .into_any_element(),
            ),
            "Combobox" => {
                let mut c = Combobox::new(
                    key.to_string(),
                    &self.field(node, key, view, _window, cx),
                    choices(node),
                );
                if let Some(h) = change_str(view, node, "change", cx) {
                    c = c.on_change(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(c.into_any_element())
            }
            "FontPicker" => Some(
                FontPicker::new(key.to_string(), &self.field(node, key, view, _window, cx))
                    .into_any_element(),
            ),
            "CronEditor" => Some(
                CronEditor::new(key.to_string(), &self.field(node, key, view, _window, cx))
                    .into_any_element(),
            ),
            "TimezoneSelect" => Some(
                TimezoneSelect::new(key.to_string(), &self.field(node, key, view, _window, cx))
                    .into_any_element(),
            ),
            "InputGroup" => {
                let mut g = InputGroup::new(&self.field(node, key, view, _window, cx));
                for entry in prop_list(node, "before") {
                    g = g.before(InputAddon::text(entry));
                }
                for entry in prop_list(node, "after") {
                    g = g.after(InputAddon::text(entry));
                }
                for (i, child) in node.children.iter().enumerate() {
                    let addon = InputAddon::new(self.child_view(child, &format!("{key}/{i}"), cx));
                    g = if prop_str(child, "slot") == "after" {
                        g.after(addon)
                    } else {
                        g.before(addon)
                    };
                }
                Some(g.into_any_element())
            }
            "CheckboxCard" => {
                let mut c = CheckboxCard::new(
                    key.to_string(),
                    prop_bool(node, "checked"),
                    prop_str(node, "title"),
                );
                if let Some(h) = change(view, node, cx) {
                    c = c.on_change(move |on, w, cx| h(Json::Bool(on), w, cx));
                }
                Some(c.into_any_element())
            }
            "CheckboxGroup" => {
                let mut g = CheckboxGroup::new(key.to_string(), choices(node));
                if let Some(h) = change_str(view, node, "change", cx) {
                    g = g.on_change(move |v, w, cx| h(v.join(","), w, cx));
                }
                Some(g.into_any_element())
            }
            "RadioGroup" => {
                let mut g = RadioGroup::new(key.to_string(), choices(node));
                if let Some(h) = change_str(view, node, "change", cx) {
                    g = g.on_change(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(g.into_any_element())
            }
            "ListBox" => {
                let mut l = ListBox::new(key.to_string(), choices(node));
                if let Some(h) = change_str(view, node, "change", cx) {
                    l = l.on_change(move |v, w, cx| h(v.join(","), w, cx));
                }
                Some(l.into_any_element())
            }
            "Cascader" => {
                let mut roots: Vec<Cascade> = Vec::new();
                for path in prop_list(node, "tree") {
                    let mut cursor = &mut roots;
                    for part in path.split('/').filter(|p| !p.is_empty()) {
                        let pos = match cursor.iter().position(|c| c.value == part) {
                            Some(i) => i,
                            None => {
                                cursor.push(Cascade::new(part.trim(), part.trim()));
                                cursor.len() - 1
                            }
                        };
                        cursor = &mut cursor[pos].children;
                    }
                }
                let mut c = Cascader::new(key.to_string(), roots);
                if let Some(h) = change_str(view, node, "change", cx) {
                    c = c.on_change(move |v, w, cx| h(v.join("/"), w, cx));
                }
                Some(c.into_any_element())
            }
            "RangeSlider" => {
                let mut s = RangeSlider::new(
                    key.to_string(),
                    prop_num(node, "low").unwrap_or(0.0),
                    prop_num(node, "high").unwrap_or(100.0),
                );
                if let Some(h) = change(view, node, cx) {
                    s = s.on_change(move |(lo, hi), w, cx| {
                        h(
                            Json::Array(vec![
                                serde_json::Number::from_f64(lo)
                                    .map(Json::Number)
                                    .unwrap_or(Json::Null),
                                serde_json::Number::from_f64(hi)
                                    .map(Json::Number)
                                    .unwrap_or(Json::Null),
                            ]),
                            w,
                            cx,
                        )
                    });
                }
                Some(s.into_any_element())
            }
            "TagInput" => {
                let mut t = TagInput::new(key.to_string(), prop_list(node, "tags"));
                if let Some(p) = prop_str_opt(node, "placeholder") {
                    t = t.placeholder(p);
                }
                t = t.suggestions(choices(node));
                if let Some(h) = change_str(view, node, "change", cx) {
                    t = t.on_change(move |v, w, cx| {
                        h(
                            v.iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                            w,
                            cx,
                        )
                    });
                }
                Some(t.into_any_element())
            }
            "TransferList" => {
                let mut t = TransferList::new(key.to_string(), choices(node));
                if let Some(h) = change_str(view, node, "change", cx) {
                    t = t.on_change(move |v, w, cx| h(v.join(","), w, cx));
                }
                Some(t.into_any_element())
            }
            "UnitInput" => {
                let units = prop_list(node, "units");
                let unit = prop_str(node, "unit");
                let mut u = UnitInput::new(
                    key.to_string(),
                    prop_num(node, "value").unwrap_or(0.0),
                    units.iter().cloned(),
                    if unit.is_empty() {
                        units.first().cloned().unwrap_or_default()
                    } else {
                        unit
                    },
                );
                if let Some(h) = change(view, node, cx) {
                    u = u.on_change(move |v, w, cx| {
                        h(
                            serde_json::Number::from_f64(v)
                                .map(Json::Number)
                                .unwrap_or(Json::Null),
                            w,
                            cx,
                        )
                    });
                }
                if let Some(h) = change_str(view, node, "unit", cx) {
                    u = u.on_unit(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(u.into_any_element())
            }
            "FieldArray" => {
                let mut f = FieldArray::new(key.to_string());
                if let Some(l) = prop_str_opt(node, "add_label") {
                    f = f.add_label(l);
                }
                for v in self.child_views(node, key, cx) {
                    f = f.row(v);
                }
                if let Some(h) = handler(view, node, "add", cx) {
                    f = f.on_add(h);
                }
                if let Some(h) = change(view, node, cx) {
                    f = f.on_remove(move |i, w, cx| h(Json::from(i), w, cx));
                }
                Some(f.into_any_element())
            }
            "KeyValueInput" => {
                let seed = prop_list(node, "pairs").into_iter().filter_map(|e| {
                    let mut p = e.splitn(2, '|');
                    Some((
                        p.next()?.trim().to_owned(),
                        p.next().unwrap_or("").trim().to_owned(),
                    ))
                });
                let mut k = KeyValueInput::new(key.to_string(), seed);
                if let Some(h) = change_str(view, node, "change", cx) {
                    k = k.on_change(move |v, w, cx| h(format!("{v:?}"), w, cx));
                }
                Some(k.into_any_element())
            }
            "ListInput" => {
                let mut l = ListInput::new(key.to_string(), prop_list(node, "items"));
                if let Some(h) = change_str(view, node, "change", cx) {
                    l = l.on_change(move |v, w, cx| h(v.join(","), w, cx));
                }
                Some(l.into_any_element())
            }
            "FileInput" => {
                let paths = prop_list(node, "paths")
                    .into_iter()
                    .map(std::path::PathBuf::from);
                let mut f = FileInput::new(key.to_string(), paths);
                if let Some(h) = change_str(view, node, "change", cx) {
                    f = f.on_change(move |v, w, cx| {
                        h(
                            v.iter()
                                .map(|p: &std::path::PathBuf| p.display().to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                            w,
                            cx,
                        )
                    });
                }
                Some(f.into_any_element())
            }
            "DropZone" => {
                let mut d = DropZone::new(key.to_string());
                if let Some(h) = change_str(view, node, "drop", cx) {
                    d = d.on_drop(move |v, w, cx| {
                        h(
                            v.iter()
                                .map(|p: &std::path::PathBuf| p.display().to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                            w,
                            cx,
                        )
                    });
                }
                Some(d.into_any_element())
            }
            "UploadList" => {
                let uploads = prop_list(node, "uploads").into_iter().filter_map(|e| {
                    let mut p = e.splitn(3, '|');
                    let (name, bytes, raw) = (
                        p.next()?.trim().to_owned(),
                        p.next()
                            .and_then(|b| b.trim().parse::<u64>().ok())
                            .unwrap_or(0),
                        p.next().unwrap_or("0").trim().to_owned(),
                    );
                    let state = if let Some(why) = raw.strip_prefix("failed:") {
                        UploadState::Failed(why.to_owned().into())
                    } else if raw == "done" {
                        UploadState::Done
                    } else {
                        UploadState::Uploading(raw.parse().unwrap_or(0.0))
                    };
                    Some(Upload {
                        name: name.into(),
                        bytes,
                        state,
                    })
                });
                let mut u = UploadList::new(key.to_string(), uploads);
                if let Some(h) = handler(view, node, "cancel", cx) {
                    u = u.on_cancel(move |i, w, cx| {
                        let _ = i;
                        h(w, cx)
                    });
                }
                if let Some(h) = handler(view, node, "retry", cx) {
                    u = u.on_retry(move |i, w, cx| {
                        let _ = i;
                        h(w, cx)
                    });
                }
                if let Some(h) = handler(view, node, "remove", cx) {
                    u = u.on_remove(move |i, w, cx| {
                        let _ = i;
                        h(w, cx)
                    });
                }
                Some(u.into_any_element())
            }
            "AvatarUpload" => {
                let mut a = AvatarUpload::new(
                    key.to_string(),
                    Avatar::new(format!("{key}/avatar"), prop_str(node, "name")),
                );
                if let Some(h) = change_str(view, node, "change", cx) {
                    a = a.on_change(move |v, w, cx| h(v.display().to_string(), w, cx));
                }
                Some(a.into_any_element())
            }
            "SignaturePad" => {
                Some(SignaturePad::new(key.to_string(), Vec::new()).into_any_element())
            }
            "IconPicker" => {
                let mut p = IconPicker::new(key.to_string());
                if let Some(i) = prop_str_opt(node, "value").and_then(|n| icon_named(&n)) {
                    p = p.selected(i);
                }
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |i, w, cx| h(i.name().to_string(), w, cx));
                }
                Some(p.into_any_element())
            }
            "EmojiPicker" => {
                let mut p = EmojiPicker::new(key.to_string());
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(p.into_any_element())
            }
            "EyeDropper" => {
                let mut d = EyeDropper::new(key.to_string());
                if let Some(h) = change_str(view, node, "pick", cx) {
                    d = d.on_pick(move |v, w, cx| h(ely_hex(v.into()), w, cx));
                }
                Some(d.into_any_element())
            }
            "ColorPicker" => {
                let color: Hsla = parse_hex(&prop_str(node, "value"))
                    .map(Into::into)
                    .unwrap_or_else(|_| hsla(0.0, 0.0, 1.0, 1.0));
                let mut p = ColorPicker::new(key.to_string(), color);
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |v, w, cx| h(ely_hex(v.into()), w, cx));
                }
                Some(p.into_any_element())
            }
            "ColorSwatch" => {
                let color: Hsla = parse_hex(&prop_str(node, "color"))
                    .map(Into::into)
                    .unwrap_or_else(|_| hsla(0.0, 0.0, 0.5, 1.0));
                Some(ColorSwatch::new(key.to_string(), color).into_any_element())
            }
            "GradientEditor" => {
                let stops = prop_list(node, "stops").into_iter().filter_map(|e| {
                    let mut p = e.splitn(2, '|');
                    let at = p.next()?.trim().parse::<f32>().ok()?;
                    let color: Hsla = parse_hex(p.next()?.trim()).map(Into::into).ok()?;
                    Some(GradientStop { at, color })
                });
                let mut g = GradientEditor::new(key.to_string(), stops);
                if let Some(h) = change_str(view, node, "change", cx) {
                    g = g.on_change(move |v, w, cx| {
                        h(
                            v.iter()
                                .map(|s: &GradientStop| {
                                    format!("{}|{}", s.at, ely_hex(s.color.into()))
                                })
                                .collect::<Vec<_>>()
                                .join(","),
                            w,
                            cx,
                        )
                    });
                }
                Some(g.into_any_element())
            }
            "Calendar" => {
                let mut c = Calendar::new(key.to_string());
                if let Ok(d) = prop_str(node, "selected").parse::<Date>() {
                    c = c.selected(d);
                }
                if let Some(h) = change_str(view, node, "pick", cx) {
                    c = c.on_pick(move |d, w, cx| h(d.to_string(), w, cx));
                }
                Some(c.into_any_element())
            }
            "DatePicker" => {
                let mut p = DatePicker::new(
                    key.to_string(),
                    prop_str(node, "value").parse::<Date>().ok(),
                );
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |d, w, cx| h(d.to_string(), w, cx));
                }
                Some(p.into_any_element())
            }
            "DateRangePicker" => {
                let value = match (
                    prop_str(node, "start").parse::<Date>(),
                    prop_str(node, "end").parse::<Date>(),
                ) {
                    (Ok(a), Ok(b)) => Some((a, b)),
                    _ => None,
                };
                let mut p = DateRangePicker::new(key.to_string(), value);
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |(a, b), w, cx| h(format!("{a}|{b}"), w, cx));
                }
                Some(p.into_any_element())
            }
            "WeekPicker" => {
                let mut p = WeekPicker::new(
                    key.to_string(),
                    prop_num(node, "year")
                        .and_then(|y| prop_num(node, "week").map(|w| (y as i16, w as i8))),
                );
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |(y, wk), w, cx| h(format!("{y}|{wk}"), w, cx));
                }
                Some(p.into_any_element())
            }
            "MonthPicker" => {
                let mut p = MonthPicker::new(
                    key.to_string(),
                    prop_num(node, "year")
                        .and_then(|y| prop_num(node, "month").map(|m| (y as i16, m as i8))),
                );
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |(y, m), w, cx| h(format!("{y}|{m}"), w, cx));
                }
                Some(p.into_any_element())
            }
            "QuarterPicker" => {
                let mut p = QuarterPicker::new(
                    key.to_string(),
                    prop_num(node, "year")
                        .and_then(|y| prop_num(node, "quarter").map(|q| (y as i16, q as i8))),
                );
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |(y, q), w, cx| h(format!("{y}|{q}"), w, cx));
                }
                Some(p.into_any_element())
            }
            "YearPicker" => {
                let mut p =
                    YearPicker::new(key.to_string(), prop_num(node, "year").map(|y| y as i16));
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |y, w, cx| h(y.to_string(), w, cx));
                }
                Some(p.into_any_element())
            }
            "TimePicker" => {
                let mut p = TimePicker::new(
                    key.to_string(),
                    prop_str(node, "value").parse::<Time>().ok(),
                );
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |t, w, cx| h(t.to_string(), w, cx));
                }
                Some(p.into_any_element())
            }
            "DateTimePicker" => {
                let mut p = DateTimePicker::new(
                    key.to_string(),
                    prop_str(node, "value").parse::<DateTime>().ok(),
                );
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |t, w, cx| h(t.to_string(), w, cx));
                }
                Some(p.into_any_element())
            }
            "DurationPicker" => {
                let mut p = DurationPicker::new(
                    key.to_string(),
                    Duration::from_secs_f64(prop_num(node, "seconds").unwrap_or(0.0).max(0.0)),
                );
                if let Some(h) = change(view, node, cx) {
                    p = p.on_change(move |d, w, cx| {
                        h(
                            serde_json::Number::from_f64(d.as_secs_f64())
                                .map(Json::Number)
                                .unwrap_or(Json::Null),
                            w,
                            cx,
                        )
                    });
                }
                Some(p.into_any_element())
            }
            "RelativeDatePicker" => {
                let range = match prop_str(node, "range").as_str() {
                    "Yesterday" => Some(RelativeRange::Yesterday),
                    "Last7Days" => Some(RelativeRange::Last7Days),
                    "Last30Days" => Some(RelativeRange::Last30Days),
                    "ThisMonth" => Some(RelativeRange::ThisMonth),
                    "LastMonth" => Some(RelativeRange::LastMonth),
                    "Today" => Some(RelativeRange::Today),
                    _ => None,
                };
                let mut p = RelativeDatePicker::new(key.to_string(), range);
                if let Some(h) = change_str(view, node, "change", cx) {
                    p = p.on_change(move |r, (a, b), w, cx| h(format!("{r:?}|{a}|{b}"), w, cx));
                }
                Some(p.into_any_element())
            }
            "ScrubInput" => {
                let mut s = ScrubInput::new(
                    key.to_string(),
                    prop_str(node, "label"),
                    prop_num(node, "value").unwrap_or(0.0),
                );
                if let Some(h) = change(view, node, cx) {
                    s = s.on_change(move |v, w, cx| {
                        h(
                            serde_json::Number::from_f64(v)
                                .map(Json::Number)
                                .unwrap_or(Json::Null),
                            w,
                            cx,
                        )
                    });
                }
                Some(s.into_any_element())
            }
            "FormSection" => {
                let mut f = FormSection::new(prop_str(node, "title"));
                f.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(f.into_any_element())
            }
            "FormLabel" => {
                Some(FormLabel::new(key.to_string(), prop_str(node, "text")).into_any_element())
            }
            "FormError" => {
                Some(FormError::new(key.to_string(), prop_str(node, "text")).into_any_element())
            }
            "FormField" => {
                let mut f = FormField::new(key.to_string(), prop_str(node, "label"));
                f.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(f.into_any_element())
            }
            "InlineForm" => {
                let mut f = InlineForm::new();
                f.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(f.into_any_element())
            }
            "DirtyIndicator" => Some(
                DirtyIndicator::new(key.to_string(), prop_bool(node, "dirty")).into_any_element(),
            ),
            "ListItem" => {
                let mut item = ListItem::new(key.to_string(), prop_str(node, "title"));
                if let Some(d) = prop_str_opt(node, "description") {
                    item = item.description(d);
                }
                if let Some(d) = prop_str_opt(node, "detail") {
                    item = item.detail(d);
                }
                item = item
                    .selected(prop_bool(node, "selected"))
                    .current(prop_bool(node, "current"))
                    .disabled(prop_bool(node, "disabled"));
                if let Some(h) = fire_str(view, node, "secondary", cx) {
                    item =
                        item.on_secondary(move |pt, w, cx| h(format!("{}|{}", pt.x, pt.y), w, cx));
                }
                Some(item.into_any_element())
            }
            "List" => {
                let mut l = List::new();
                l.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(l.into_any_element())
            }
            "GroupedList" => {
                let mut g = GroupedList::new(key.to_string());
                let mut groups: Vec<(String, Vec<AnyView>)> = Vec::new();
                for (i, child) in node.children.iter().enumerate() {
                    let title = prop_str(child, "group");
                    let v = self.child_view(child, &format!("{key}/{i}"), cx);
                    match groups.last_mut() {
                        Some((t, rows)) if *t == title => rows.push(v),
                        _ => groups.push((title, vec![v])),
                    }
                }
                for (title, rows) in groups {
                    g = g.group(title, rows);
                }
                Some(g.into_any_element())
            }
            "InfiniteList" => {
                let mut l = InfiniteList::new(key.to_string());
                l.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                if let Some(h) = handler(view, node, "more", cx) {
                    l = l.on_more(h);
                }
                Some(l.into_any_element())
            }
            "VirtualList" => {
                let rows: Rc<Vec<AnyView>> = Rc::new(self.child_views(node, key, cx));
                let count = prop_num(node, "count")
                    .map(|c| c.max(0.0) as usize)
                    .unwrap_or_else(|| rows.len().max(1));
                let list = VirtualList::new(key.to_string(), count, move |i, _w, _cx| {
                    match rows.get(i) {
                        Some(v) => v.clone().into_any_element(),
                        None => gpui::div().child(format!("Row {i}")).into_any_element(),
                    }
                });
                Some(list.into_any_element())
            }
            "Outline" => {
                let mut o = Outline::new(key.to_string());
                for entry in prop_list(node, "items") {
                    let mut p = entry.splitn(2, '|');
                    let level: usize = p.next().and_then(|l| l.trim().parse().ok()).unwrap_or(1);
                    let title = p.next().unwrap_or("").trim();
                    o = o.item(title, title, level.max(1));
                }
                if let Some(c) = prop_str_opt(node, "current") {
                    o = o.current(c);
                }
                if let Some(h) = change_str(view, node, "select", cx) {
                    o = o.on_select(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(o.into_any_element())
            }
            "Sections" => {
                let mut sec = Sections::new(key.to_string()).counted();
                let mut groups: Vec<(String, Vec<(SharedString, ListItem)>)> = Vec::new();
                for (i, child) in node.children.iter().enumerate() {
                    let title = prop_str(child, "group");
                    let name = prop_str(child, "title");
                    let name = if name.is_empty() {
                        prop_str(child, "label")
                    } else {
                        name
                    };
                    let mut item = ListItem::new(format!("{key}/{i}"), name.clone());
                    if let Some(d) = prop_str_opt(child, "description") {
                        item = item.description(d);
                    }
                    if let Some(d) = prop_str_opt(child, "detail") {
                        item = item.detail(d);
                    }
                    let row = (SharedString::from(name), item);
                    match groups.last_mut() {
                        Some((t, rows)) if *t == title => rows.push(row),
                        _ => groups.push((title, vec![row])),
                    }
                }
                for (title, rows) in groups {
                    sec = sec.section(title.into(), rows);
                }
                if let Some(h) = change_str(view, node, "select", cx) {
                    sec = sec.on_select(Rc::new(move |v: &SharedString, w, cx| {
                        h(v.to_string(), w, cx)
                    }));
                }
                if let Some(h) = fire_str(view, node, "activate", cx) {
                    sec = sec.on_activate(Rc::new(move |v: &SharedString, w, cx| {
                        h(v.to_string(), w, cx)
                    }));
                }
                Some(sec.into_any_element())
            }
            "SelectableList" => {
                let mut l = SelectableList::new(key.to_string());
                for (i, name) in prop_list(node, "options").into_iter().enumerate() {
                    l = l.row(name.clone(), ListItem::new(format!("{key}/{i}"), name));
                }
                if let Some(h) = fire_str(view, node, "activate", cx) {
                    l = l.on_activate(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = change_str(view, node, "change", cx) {
                    l = l.on_change(move |v, w, cx| {
                        h(
                            v.iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                            w,
                            cx,
                        )
                    });
                }
                Some(l.into_any_element())
            }
            "SortableList" => {
                let mut l = SortableList::new(key.to_string());
                for (i, name) in prop_list(node, "options").into_iter().enumerate() {
                    l = l.row(name.clone(), ListItem::new(format!("{key}/{i}"), name));
                }
                if let Some(h) = change_str(view, node, "reorder", cx) {
                    l = l.on_reorder(move |from, to, w, cx| h(format!("{from}|{to}"), w, cx));
                }
                Some(l.into_any_element())
            }
            "SwipeableListItem" => {
                let item = self
                    .child_views(node, key, cx)
                    .into_iter()
                    .next()
                    .map(|v| v.into_any_element())
                    .unwrap_or_else(|| {
                        gpui::div()
                            .child(prop_str(node, "title"))
                            .into_any_element()
                    });
                let mut s = SwipeableListItem::new(key.to_string(), item);
                for action in prop_list(node, "actions") {
                    let mut p = action.splitn(3, '|');
                    let label = p.next().unwrap_or("").trim().to_owned();
                    let icon = p
                        .next()
                        .and_then(|n| icon_named(n.trim()))
                        .unwrap_or(IconName::Check);
                    let tone = p.next().and_then(|t| match t.trim() {
                        "Accent" => Some(Tone::Accent),
                        "Info" => Some(Tone::Info),
                        "Success" => Some(Tone::Success),
                        "Warning" => Some(Tone::Warning),
                        "Danger" => Some(Tone::Danger),
                        _ => None,
                    });
                    let run = fire_str(view, node, &format!("action:{label}"), cx);
                    let mut a = SwipeAction::new(label, icon, move |w, cx| {
                        if let Some(run) = &run {
                            run(String::new(), w, cx);
                        }
                    });
                    if let Some(t) = tone {
                        a = a.tone(t);
                    }
                    s = s.action(a);
                }
                Some(s.into_any_element())
            }
            "DirectoryListing" => {
                let path = prop_list(node, "path");
                let path = if path.is_empty() {
                    vec![".".to_owned()]
                } else {
                    path
                };
                let entries = prop_list(node, "entries").into_iter().filter_map(|e| {
                    let mut p = e.splitn(3, '|');
                    let name = p.next()?.trim().to_owned();
                    let size = p.next().unwrap_or("").trim();
                    let _modified = p.next();
                    Some(if size.is_empty() {
                        DirEntry::folder(name, Timestamp::now())
                    } else {
                        DirEntry::file(name, size.parse().unwrap_or(0), Timestamp::now())
                    })
                });
                let mut d = DirectoryListing::new(key.to_string(), path, entries);
                if let Some(sel) = prop_str_opt(node, "selected") {
                    d = d.selected(sel);
                }
                if let Some(h) = change_str(view, node, "select", cx) {
                    d = d.on_select(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "open", cx) {
                    d = d.on_open(move |e, w, cx| h(e.name().to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "climb", cx) {
                    d = d.on_climb(move |i, w, cx| h(i.to_string(), w, cx));
                }
                Some(d.into_any_element())
            }
            "FileTree" => {
                let mut f = FileTree::new(key.to_string(), prop_list(node, "paths"));
                for entry in prop_list(node, "status") {
                    let mut p = entry.splitn(2, '|');
                    let (path, st) = (p.next().unwrap_or("").trim(), p.next().unwrap_or("").trim());
                    let status = match st {
                        "Added" => GitStatus::Added,
                        "Deleted" => GitStatus::Deleted,
                        "Untracked" => GitStatus::Untracked,
                        "Renamed" => GitStatus::Renamed,
                        "Conflicted" => GitStatus::Conflicted,
                        _ => GitStatus::Modified,
                    };
                    f = f.status(path, status);
                }
                if let Some(h) = fire_str(view, node, "open", cx) {
                    f = f.on_open(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "context", cx) {
                    f = f.on_context(move |v, _pt, w, cx| h(v.to_string(), w, cx));
                }
                Some(f.into_any_element())
            }
            "Tree" => {
                let mut t = Tree::new(key.to_string(), tree_nodes(&prop_list(node, "tree")));
                t = t
                    .open(prop_list(node, "open"))
                    .selected(prop_list(node, "selected"));
                if prop_bool(node, "multiple") {
                    t = t.multiple();
                }
                if let Some(h) = change_str(view, node, "select", cx) {
                    t = t.on_select(move |v: &[SharedString], w, cx| {
                        h(
                            v.iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                            w,
                            cx,
                        )
                    });
                }
                if let Some(h) = fire_str(view, node, "activate", cx) {
                    t = t.on_activate(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(t.into_any_element())
            }
            "TreeSelect" => {
                let mut t = TreeSelect::new(key.to_string(), tree_nodes(&prop_list(node, "tree")));
                if let Some(h) = change_str(view, node, "change", cx) {
                    t = t.on_change(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(t.into_any_element())
            }
            "DropdownMenu" => {
                let mut d = DropdownMenu::new(
                    key.to_string(),
                    prop_str(node, "label"),
                    menu_of(node, view, cx),
                );
                if let Some(i) = prop_str_opt(node, "icon").and_then(|n| icon_named(&n)) {
                    d = d.icon(i);
                }
                Some(d.into_any_element())
            }
            "OverflowMenu" => {
                Some(OverflowMenu::new(key.to_string(), menu_of(node, view, cx)).into_any_element())
            }
            "ContextMenu" => {
                let mut c = ContextMenu::new(key.to_string(), menu_of(node, view, cx));
                c.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(c.into_any_element())
            }
            "SearchableMenu" => Some(
                SearchableMenu::new(
                    key.to_string(),
                    prop_str(node, "label"),
                    menu_of(node, view, cx),
                )
                .into_any_element(),
            ),
            "SplitButton" => {
                let mut b = SplitButton::new(
                    key.to_string(),
                    prop_str(node, "label"),
                    menu_of(node, view, cx),
                );
                if let Some(h) = handler(view, node, "click", cx) {
                    b = b.on_click(move |_e: &ClickEvent, w, cx| h(w, cx));
                }
                Some(b.into_any_element())
            }
            "PieMenu" => {
                let items = prop_list(node, "items").into_iter().filter_map(|e| {
                    let mut p = e.splitn(2, '|');
                    let label = p.next()?.trim().to_owned();
                    let icon = p
                        .next()
                        .and_then(|n| icon_named(n.trim()))
                        .unwrap_or(IconName::Circle);
                    let run = fire_str(view, node, &format!("item:{label}"), cx);
                    Some(PieItem::new(icon, label).on_click(move |w, cx| {
                        if let Some(run) = &run {
                            run(String::new(), w, cx);
                        }
                    }))
                });
                Some(PieMenu::new(key.to_string(), items).into_any_element())
            }
            "Breadcrumb" => {
                let crumbs = prop_list(node, "crumbs").into_iter().filter_map(|e| {
                    let mut p = e.splitn(3, '|');
                    let (value, label) = (p.next()?.trim(), p.next().unwrap_or("").trim());
                    let mut c = Crumb::new(value, label);
                    if let Some(i) = p.next().and_then(|n| icon_named(n.trim())) {
                        c = c.icon(i);
                    }
                    Some(c)
                });
                let mut b = Breadcrumb::new(key.to_string(), crumbs);
                if let Some(h) = change_str(view, node, "select", cx) {
                    b = b.on_select(move |i, v, w, cx| h(format!("{i}|{v}"), w, cx));
                }
                Some(b.into_any_element())
            }
            "EditorTabs" => {
                let tabs = prop_list(node, "tabs").into_iter().filter_map(|e| {
                    let mut p = e.splitn(3, '|');
                    let (id, title) = (p.next()?.trim(), p.next().unwrap_or("").trim());
                    let mut t = EditorTab::new(id, title);
                    for flag in p.next().unwrap_or("").split(',') {
                        match flag.trim() {
                            "dirty" => t = t.dirty(true),
                            "pinned" => t = t.pinned(true),
                            "preview" => t = t.preview(true),
                            i if icon_named(i).is_some() => t = t.icon(icon_named(i).unwrap()),
                            _ => {}
                        }
                    }
                    Some(t)
                });
                let mut t = EditorTabs::new(key.to_string(), tabs);
                if let Some(sel) = prop_str_opt(node, "selected") {
                    t = t.selected(sel);
                }
                if let Some(h) = change_str(view, node, "select", cx) {
                    t = t.on_select(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "close", cx) {
                    t = t.on_close(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "pin", cx) {
                    t = t.on_pin(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "keep", cx) {
                    t = t.on_keep(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "context", cx) {
                    t = t.on_context(move |v, _pt, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "reorder", cx) {
                    t = t.on_reorder(move |a, b, w, cx| h(format!("{a}|{b}"), w, cx));
                }
                Some(t.into_any_element())
            }
            "NavigationMenu" => {
                let mut n = NavigationMenu::new(key.to_string());
                for entry in prop_list(node, "entries") {
                    let mut p = entry.splitn(2, '|');
                    let label = p.next().unwrap_or("").trim().to_owned();
                    let links = p
                        .next()
                        .unwrap_or("")
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(|s| Choice::new(s, s));
                    n = n.entry(label, links);
                }
                if let Some(h) = change_str(view, node, "select", cx) {
                    n = n.on_select(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(n.into_any_element())
            }
            "TabOverflowMenu" => {
                let mut t = TabOverflowMenu::new(key.to_string(), choices(node));
                if let Some(sel) = prop_str_opt(node, "selected") {
                    t = t.selected(sel);
                }
                if let Some(h) = change_str(view, node, "select", cx) {
                    t = t.on_select(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(t.into_any_element())
            }
            "Steps" => {
                let mut st = Steps::new(
                    key.to_string(),
                    choices(node),
                    prop_num(node, "current").unwrap_or(0.0) as usize,
                );
                if prop_bool(node, "vertical") {
                    st = st.vertical();
                }
                if let Some(h) = fire_str(view, node, "select", cx) {
                    st = st.on_select(move |i, w, cx| h(i.to_string(), w, cx));
                }
                Some(st.into_any_element())
            }
            "Wizard" => {
                let on_step = {
                    let h = fire_str(view, node, "step", cx);
                    move |i: usize, w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(i.to_string(), w, cx)
                        }
                    }
                };
                let on_finish = {
                    let h = handler(view, node, "finish", cx);
                    move |w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(w, cx)
                        }
                    }
                };
                let mut w = Wizard::new(
                    key.to_string(),
                    choices(node),
                    prop_num(node, "current").unwrap_or(0.0) as usize,
                    on_step,
                    on_finish,
                )
                .ready(!prop_bool(node, "blocked"));
                if let Some(v) = self.child_views(node, key, cx).into_iter().next() {
                    w = w.content(v);
                }
                Some(w.into_any_element())
            }
            "Tabs" => {
                let selected = prop_str(node, "selected");
                let selected = if selected.is_empty() {
                    prop_list(node, "options")
                        .first()
                        .cloned()
                        .unwrap_or_default()
                } else {
                    selected
                };
                let mut t = Tabs::new(key.to_string(), choices(node), selected);
                t = match prop_str(node, "placement").as_str() {
                    "Left" => t.placement(TabPlacement::Left),
                    "Bottom" => t.placement(TabPlacement::Bottom),
                    _ => t,
                };
                if let Some(v) = self.child_views(node, key, cx).into_iter().next() {
                    t = t.panel(v);
                }
                if let Some(h) = change_str(view, node, "change", cx) {
                    t = t.on_change(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "context", cx) {
                    t = t.on_context(move |v, _pt, w, cx| h(v.to_string(), w, cx));
                }
                Some(t.into_any_element())
            }
            "GoToLine" => {
                let mut g = GoToLine::new(
                    key.to_string(),
                    &self.field(node, key, view, _window, cx),
                    prop_num(node, "lines").unwrap_or(1.0).max(1.0) as usize,
                );
                if let Some(h) = fire_str(view, node, "jump", cx) {
                    g = g.on_jump(move |to, from, w, cx| {
                        h(
                            format!("{to}|{}", from.map(|f| f.to_string()).unwrap_or_default()),
                            w,
                            cx,
                        )
                    });
                }
                Some(g.into_any_element())
            }
            "BackForwardNavigation" => {
                let mut n = BackForwardNavigation::new(
                    key.to_string(),
                    choices(node),
                    prop_num(node, "current").unwrap_or(0.0) as usize,
                );
                if let Some(h) = fire_str(view, node, "go", cx) {
                    n = n.on_go(move |i, w, cx| h(i.to_string(), w, cx));
                }
                Some(n.into_any_element())
            }
            "LoadMore" => {
                let mut l = LoadMore::new(key.to_string(), prop_bool(node, "loading"));
                let (shown, total) = (
                    prop_num(node, "shown").unwrap_or(0.0) as usize,
                    prop_num(node, "total").unwrap_or(0.0) as usize,
                );
                if total > 0 {
                    l = l.shown(shown, total);
                }
                if let Some(h) = handler(view, node, "load", cx) {
                    l = l.on_load(h);
                }
                Some(l.into_any_element())
            }
            "NavItem" => {
                let mut n = NavItem::new(key.to_string(), icon_of(node), prop_str(node, "label"));
                if let Some(c) = prop_str_opt(node, "count") {
                    n = n.count(c);
                }
                n = n
                    .active(prop_bool(node, "active"))
                    .folded(prop_bool(node, "folded"));
                if let Some(h) = handler(view, node, "click", cx) {
                    n = n.on_click(h);
                }
                Some(n.into_any_element())
            }
            "NavGroup" => {
                let mut g = NavGroup::new(
                    key.to_string(),
                    prop_str(node, "title"),
                    prop_bool(node, "open"),
                )
                .folded(prop_bool(node, "folded"));
                if let Some(h) = fire_str(view, node, "toggle", cx) {
                    g = g.on_toggle(move |open, w, cx| h(open.to_string(), w, cx));
                }
                Some(g.into_any_element())
            }
            "TableOfContents" => {
                let name = prop_str_opt(node, "toc").unwrap_or_else(|| key.to_owned());
                let mut t = TableOfContents::new(key.to_string(), &self.toc(&name, cx));
                for entry in prop_list(node, "items") {
                    let mut p = entry.splitn(3, '|');
                    let depth: usize = p.next().and_then(|d| d.trim().parse().ok()).unwrap_or(0);
                    let (value, label) =
                        (p.next().unwrap_or("").trim(), p.next().unwrap_or("").trim());
                    t = t.entry(value, label, depth);
                }
                Some(t.into_any_element())
            }
            "Anchor" => {
                let name = prop_str_opt(node, "toc").unwrap_or_else(|| key.to_owned());
                let mut a = Anchor::new(&self.toc(&name, cx), prop_str(node, "value"));
                a.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(a.into_any_element())
            }
            "Dialog" => {
                let on_close = {
                    let h = handler(view, node, "close", cx);
                    move |w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(w, cx)
                        }
                    }
                };
                let mut d = Dialog::new(key.to_string(), prop_str(node, "title"), on_close);
                if let Some(t) = prop_str_opt(node, "detail") {
                    d = d.detail(t);
                }
                for label in prop_list(node, "actions") {
                    let run = fire_str(view, node, &format!("action:{label}"), cx).map(Rc::new);
                    let id = format!("{key}/action");
                    d = d.action(move |close| {
                        let run = run.clone();
                        Button::new(id.clone(), label.clone()).on_click(
                            move |_e: &ClickEvent, w, cx| {
                                if let Some(run) = &run {
                                    run(String::new(), w, cx);
                                }
                                close(w, cx);
                            },
                        )
                    });
                }
                d.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(d.into_any_element())
            }
            "AlertDialog" => {
                let on_close = {
                    let h = handler(view, node, "close", cx);
                    move |w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(w, cx)
                        }
                    }
                };
                let mut d = AlertDialog::new(
                    key.to_string(),
                    severity_of(node),
                    prop_str(node, "title"),
                    prop_str(node, "message"),
                    on_close,
                );
                if let Some(l) = prop_str_opt(node, "label") {
                    d = d.label(l);
                }
                Some(d.into_any_element())
            }
            "ConfirmDialog" => {
                let on_close = {
                    let h = handler(view, node, "close", cx);
                    move |w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(w, cx)
                        }
                    }
                };
                let mut d = ConfirmDialog::new(
                    key.to_string(),
                    prop_str(node, "title"),
                    prop_str(node, "message"),
                    on_close,
                );
                if let Some(l) = prop_str_opt(node, "confirm") {
                    d = d.confirm(l);
                }
                if prop_bool(node, "destructive") {
                    d = d.destructive();
                }
                if let Some(h) = handler(view, node, "confirm", cx) {
                    d = d.on_confirm(h);
                }
                Some(d.into_any_element())
            }
            "PromptDialog" => {
                let on_close = {
                    let h = handler(view, node, "close", cx);
                    move |w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(w, cx)
                        }
                    }
                };
                let mut d = PromptDialog::new(
                    key.to_string(),
                    prop_str(node, "title"),
                    &self.field(node, key, view, _window, cx),
                    on_close,
                );
                if let Some(l) = prop_str_opt(node, "label") {
                    d = d.label(l);
                }
                if let Some(l) = prop_str_opt(node, "submit") {
                    d = d.submit(l);
                }
                if let Some(h) = change_str(view, node, "submit", cx) {
                    d = d.on_submit(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(d.into_any_element())
            }
            "FloatingToolbar" => {
                let mut f = FloatingToolbar::new(
                    key.to_string(),
                    &self.field(node, key, view, _window, cx),
                );
                f.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(f.into_any_element())
            }
            "Peek" => {
                let on_close = {
                    let h = handler(view, node, "close", cx);
                    move |w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(w, cx)
                        }
                    }
                };
                let mut p = Peek::new(
                    key.to_string(),
                    prop_str(node, "title"),
                    prop_bool(node, "open"),
                    on_close,
                );
                p.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(p.into_any_element())
            }
            "HoverCard" => {
                let children = self.child_views(node, key, cx);
                let (trigger, content) = (
                    children
                        .first()
                        .map(|v| v.clone().into_any_element())
                        .unwrap_or_else(|| {
                            gpui::div()
                                .child(prop_str(node, "label"))
                                .into_any_element()
                        }),
                    children.get(1).cloned(),
                );
                let card =
                    HoverCard::new(key.to_string(), trigger, move |_w, _cx| match &content {
                        Some(v) => v.clone().into_any_element(),
                        None => gpui::div().into_any_element(),
                    });
                Some(card.into_any_element())
            }
            "Popover" => {
                let content = self.child_views(node, key, cx).into_iter().next();
                let p = Popover::new(key.to_string(), prop_str(node, "label"), move |_w, _cx| {
                    match &content {
                        Some(v) => v.clone().into_any_element(),
                        None => gpui::div().into_any_element(),
                    }
                });
                Some(p.into_any_element())
            }
            "Lightbox" => {
                let slides = prop_list(node, "slides").into_iter().filter_map(|e| {
                    let mut p = e.splitn(2, '|');
                    let source = p.next()?.trim().to_owned();
                    let caption = p.next().unwrap_or("").trim().to_owned();
                    Some(Slide::new(
                        ImageSource::from(std::path::PathBuf::from(source)),
                        caption,
                    ))
                });
                let on_close = {
                    let h = handler(view, node, "close", cx);
                    move |w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(w, cx)
                        }
                    }
                };
                let mut l = Lightbox::new(
                    key.to_string(),
                    slides,
                    prop_num(node, "at").unwrap_or(0.0) as usize,
                    on_close,
                );
                if let Some(h) = fire_str(view, node, "step", cx) {
                    l = l.on_step(move |i, w, cx| h(i.to_string(), w, cx));
                }
                Some(l.into_any_element())
            }
            "ActivityBar" => {
                let mut b = ActivityBar::new(key.to_string());
                for (i, child) in node.children.iter().enumerate() {
                    let mut item = ActivityItem::new(
                        format!("{key}/{i}"),
                        prop_str_opt(child, "icon")
                            .and_then(|n| icon_named(&n))
                            .unwrap_or(IconName::Circle),
                        prop_str(child, "label"),
                    );
                    if let Some(c) = prop_num(child, "badge") {
                        item = item.badge(c.max(0.0) as u32);
                    }
                    b = if prop_str(child, "slot") == "footer" {
                        b.footer(item)
                    } else {
                        b.item(item)
                    };
                }
                if let Some(sel) = prop_str_opt(node, "selected") {
                    b = b.selected(sel);
                }
                if let Some(h) = change_str(view, node, "select", cx) {
                    b = b.on_select(move |v, w, cx| h(v.to_string(), w, cx));
                }
                Some(b.into_any_element())
            }
            "StatusBarItem" => {
                let mut item = StatusBarItem::new(key.to_string());
                if let Some(v) = self.child_views(node, key, cx).into_iter().next() {
                    item = item.leading(v);
                }
                if let Some(i) = prop_str_opt(node, "icon").and_then(|n| icon_named(&n)) {
                    item = item.icon(i);
                }
                if let Some(l) = prop_str_opt(node, "label") {
                    item = item.label(l);
                }
                if let Some(t) = prop_str_opt(node, "tooltip") {
                    item = item.tooltip(t);
                }
                if let Some(h) = handler(view, node, "click", cx) {
                    item = item.on_click(move |_e: &ClickEvent, w, cx| h(w, cx));
                }
                Some(item.into_any_element())
            }
            "StatusBar" => {
                let mut bar = StatusBar::new();
                for (i, child) in node.children.iter().enumerate() {
                    let v = self.child_view(child, &format!("{key}/{i}"), cx);
                    bar = if prop_str(child, "slot") == "right" {
                        bar.right(v)
                    } else {
                        bar.left(v)
                    };
                }
                Some(bar.into_any_element())
            }
            "Toolbar" => {
                let mut t = Toolbar::new();
                t.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(t.into_any_element())
            }
            "ToolbarGroup" => {
                let mut g = ToolbarGroup::new();
                g.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(g.into_any_element())
            }
            "ToolbarSeparator" => Some(ToolbarSeparator.into_any_element()),
            "TitleBar" => {
                let mut t = TitleBar::new(key.to_string());
                if let Some(title) = prop_str_opt(node, "title") {
                    t = t.title(title);
                }
                if let Some(p) = platform_of(node) {
                    t = t.platform(p);
                }
                if let Some(h) = handler(view, node, "close", cx) {
                    t = t.on_close(h);
                }
                for (i, child) in node.children.iter().enumerate() {
                    let v = self.child_view(child, &format!("{key}/{i}"), cx);
                    if prop_str(child, "slot") == "action" {
                        t = t.action(v);
                    }
                }
                t.extend(
                    node.children
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| prop_str(c, "slot") != "action")
                        .map(|(i, c)| {
                            self.child_view(c, &format!("{key}/{i}"), cx)
                                .into_any_element()
                        }),
                );
                Some(t.into_any_element())
            }
            "WindowControls" => {
                let mut c = WindowControls::new(key.to_string());
                if let Some(p) = platform_of(node) {
                    c = c.platform(p);
                }
                if let Some(h) = handler(view, node, "close", cx) {
                    c = c.on_close(h);
                }
                Some(c.into_any_element())
            }
            "ColumnShell" => {
                let mut sh = ColumnShell::new(key.to_string());
                for (i, child) in node.children.iter().enumerate() {
                    let mut col = ShellColumn::new(format!("{key}/{i}"));
                    if let Some(t) = prop_str_opt(child, "title") {
                        col = col.title(t);
                    }
                    if let Some(w) = prop_num(child, "width") {
                        col = col.width(px(w as f32));
                    }
                    col.extend(std::iter::once(
                        self.child_view(child, &format!("{key}/{i}/body"), cx)
                            .into_any_element(),
                    ));
                    sh = sh.column(col);
                }
                if let Some(p) = platform_of(node) {
                    sh = sh.platform(p);
                }
                if prop_bool(node, "glass") {
                    sh = sh.glass();
                }
                if let Some(h) = handler(view, node, "close", cx) {
                    sh = sh.on_close(h);
                }
                Some(sh.into_any_element())
            }
            "ShellColumn" => {
                let mut col = ShellColumn::new(key.to_string());
                if let Some(t) = prop_str_opt(node, "title") {
                    col = col.title(t);
                }
                if let Some(w) = prop_num(node, "width") {
                    col = col.width(px(w as f32));
                }
                for (i, child) in node.children.iter().enumerate() {
                    let v = self.child_view(child, &format!("{key}/{i}"), cx);
                    match prop_str(child, "slot").as_str() {
                        "leading" => col = col.leading(v),
                        "action" => col = col.action(v),
                        _ => col.extend(std::iter::once(v.into_any_element())),
                    }
                }
                let mut sh = ColumnShell::new(format!("{key}/shell"));
                sh = sh.column(col);
                Some(sh.into_any_element())
            }
            "CrashReporter" => {
                let mut c = CrashReporter::new(
                    prop_str(node, "name"),
                    prop_str(node, "summary"),
                    prop_str(node, "report"),
                )
                .details(prop_bool(node, "details"));
                if let Some(h) = handler(view, node, "send", cx) {
                    c = c.on_send(h);
                }
                if let Some(h) = handler(view, node, "dismiss", cx) {
                    c = c.on_dismiss(h);
                }
                Some(c.into_any_element())
            }
            "OfflineIndicator" => {
                let state = match prop_str(node, "state").as_str() {
                    "Offline" => Connectivity::Offline,
                    "Reconnecting" => Connectivity::Reconnecting,
                    _ => Connectivity::Online,
                };
                Some(OfflineIndicator::new(key.to_string(), state).into_any_element())
            }
            "ZoomControl" => {
                let mut z = ZoomControl::new(key.to_string());
                if let Some(h) = change_str(view, node, "zoom", cx) {
                    let percent = prop_num(node, "percent").unwrap_or(100.0) as u32;
                    z = z.owned(percent, move |step, w, cx| {
                        h(
                            match step {
                                ZoomStep::Out => "Out",
                                ZoomStep::In => "In",
                                ZoomStep::Reset => "Reset",
                            }
                            .to_string(),
                            w,
                            cx,
                        )
                    });
                }
                Some(z.into_any_element())
            }
            "WindowSwitcher" => {
                let on_close = {
                    let h = handler(view, node, "close", cx);
                    move |w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(w, cx)
                        }
                    }
                };
                Some(WindowSwitcher::new(key.to_string(), on_close).into_any_element())
            }
            "TabBar" => {
                let mut bar = TabBar::new(key.to_string());
                for e in prop_list(node, "tabs") {
                    let mut p = e.splitn(3, '|');
                    let (id, title) =
                        (p.next().unwrap_or("").trim(), p.next().unwrap_or("").trim());
                    let mut tab = WindowTab::new(id, title);
                    if let Some(i) = p.next().and_then(|n| icon_named(n.trim())) {
                        tab = tab.icon(i);
                    }
                    bar = bar.tab(tab);
                }
                if let Some(sel) = prop_str_opt(node, "selected") {
                    bar = bar.selected(sel);
                }
                if let Some(h) = change_str(view, node, "select", cx) {
                    bar = bar.on_select(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = fire_str(view, node, "close", cx) {
                    bar = bar.on_close(move |v, w, cx| h(v.to_string(), w, cx));
                }
                if let Some(h) = handler(view, node, "add", cx) {
                    bar = bar.on_add(h);
                }
                if let Some(h) = fire_str(view, node, "reorder", cx) {
                    bar = bar.on_reorder(move |a, b, w, cx| h(format!("{a}|{b}"), w, cx));
                }
                Some(bar.into_any_element())
            }
            "UpdateBanner" => {
                let mut u = UpdateBanner::new(
                    key.to_string(),
                    prop_str(node, "version"),
                    update_state_of(node),
                );
                if let Some(h) = handler(view, node, "download", cx) {
                    u = u.on_download(h);
                }
                if let Some(h) = handler(view, node, "restart", cx) {
                    u = u.on_restart(h);
                }
                if let Some(h) = handler(view, node, "later", cx) {
                    u = u.on_later(h);
                }
                Some(u.into_any_element())
            }
            "UpdateDialog" => {
                let mut u = UpdateDialog::new(
                    prop_str(node, "name"),
                    prop_str(node, "current"),
                    prop_str(node, "version"),
                    update_state_of(node),
                );
                for note in prop_list(node, "notes") {
                    u = u.note(note);
                }
                if let Some(h) = handler(view, node, "download", cx) {
                    u = u.on_download(h);
                }
                if let Some(h) = handler(view, node, "restart", cx) {
                    u = u.on_restart(h);
                }
                if let Some(h) = handler(view, node, "later", cx) {
                    u = u.on_later(h);
                }
                Some(u.into_any_element())
            }
            "Vibrancy" => {
                let mut v = Vibrancy::new();
                v.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|e| e.into_any_element()),
                );
                Some(v.into_any_element())
            }
            "ResizeBorder" => {
                let mut r = ResizeBorder::new();
                r.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|e| e.into_any_element()),
                );
                Some(r.into_any_element())
            }
            "Table" => {
                let mut t = Table::new(prop_list(node, "headers"));
                for row in prop_list(node, "rows") {
                    let cells: Vec<String> = row.split('|').map(|c| c.trim().to_owned()).collect();
                    t = t.row(cells.into_iter().map(|c| gpui::div().child(c)));
                }
                Some(t.into_any_element())
            }
            "HeatmapTable" => {
                let mut t = HeatmapTable::new(prop_list(node, "headers"));
                for row in prop_list(node, "rows") {
                    let mut p = row.splitn(2, '|');
                    let label = p.next().unwrap_or("").trim().to_owned();
                    let values: Vec<f64> = p
                        .next()
                        .unwrap_or("")
                        .split(',')
                        .filter_map(|v| v.trim().parse().ok())
                        .collect();
                    t = t.row(label, values);
                }
                if let Some(d) = prop_num(node, "decimals") {
                    t = t.decimals(d as usize);
                }
                Some(t.into_any_element())
            }
            "DataTable" => {
                let mut t = DataTable::new(
                    key.to_string(),
                    prop_list(node, "columns").iter().map(|c| column_of(c)),
                );
                let rows: Vec<Row> = prop_list(node, "rows")
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        Row::new(
                            format!("{key}/row/{i}"),
                            r.split('|').map(|c| cell_of(c.trim())),
                        )
                    })
                    .collect();
                t = t.rows(Rc::new(rows));
                if let Some(q) = prop_str_opt(node, "query") {
                    t = t.query(q);
                }
                t = t
                    .hidden(prop_list(node, "hidden"))
                    .selected(prop_list(node, "selected"));
                if let Some(size) = prop_num(node, "page_size") {
                    t = t.paged(size.max(1.0) as usize);
                }
                if prop_bool(node, "virtualized") {
                    t = t.virtualized();
                }
                if let Some(g) = prop_str_opt(node, "group_by") {
                    t = t.group_by(g);
                }
                t = t.density(density_of(&prop_str(node, "density")));
                if let Some(h) = change_str(view, node, "select", cx) {
                    t = t.on_select(move |v, w, cx| {
                        h(
                            v.iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>()
                                .join(","),
                            w,
                            cx,
                        )
                    });
                }
                if let Some(h) = fire_str(view, node, "edit", cx) {
                    t = t.on_edit(move |r, c, v, w, cx| h(format!("{r}|{c}|{v}"), w, cx));
                }
                Some(t.into_any_element())
            }
            "DataGrid" => {
                let rows: Vec<Vec<SharedString>> = prop_list(node, "rows")
                    .iter()
                    .map(|r| {
                        r.split('|')
                            .map(|c| SharedString::from(c.trim().to_owned()))
                            .collect()
                    })
                    .collect();
                let mut g =
                    DataGrid::new(key.to_string(), prop_list(node, "headers"), Rc::new(rows));
                if let Some(h) = fire_str(view, node, "edit", cx) {
                    g = g.on_change(move |v, w, cx| {
                        h(
                            v.iter()
                                .map(|(r, c, t)| format!("{r}|{c}|{t}"))
                                .collect::<Vec<_>>()
                                .join(";"),
                            w,
                            cx,
                        )
                    });
                }
                Some(g.into_any_element())
            }
            "Spreadsheet" => {
                let mut sh = Spreadsheet::new(
                    key.to_string(),
                    prop_num(node, "rows").unwrap_or(10.0).max(1.0) as usize,
                    prop_num(node, "cols").unwrap_or(5.0).max(1.0) as usize,
                );
                let mut cells: HashMap<(usize, usize), SharedString> = HashMap::new();
                for entry in prop_list(node, "cells") {
                    if let Some((at, text)) = entry.split_once('|') {
                        let mut p = at.split(',');
                        if let (Some(r), Some(c)) = (
                            p.next().and_then(|v| v.trim().parse().ok()),
                            p.next().and_then(|v| v.trim().parse().ok()),
                        ) {
                            cells.insert((r, c), text.trim().into());
                        }
                    }
                }
                sh = sh.cells(Rc::new(cells));
                let (fr, fc) = (
                    prop_num(node, "freeze_rows").unwrap_or(0.0) as usize,
                    prop_num(node, "freeze_cols").unwrap_or(0.0) as usize,
                );
                if fr > 0 || fc > 0 {
                    sh = sh.freeze(fr, fc);
                }
                if let Some(h) = fire_str(view, node, "edit", cx) {
                    sh = sh.on_change(move |v, w, cx| {
                        h(
                            v.iter()
                                .map(|(r, c, t)| format!("{r}|{c}|{t}"))
                                .collect::<Vec<_>>()
                                .join(";"),
                            w,
                            cx,
                        )
                    });
                }
                Some(sh.into_any_element())
            }
            "PivotTable" => {
                let row_list = prop_list(node, "rows");
                let records = row_list.iter().map(|r| {
                    r.split('|')
                        .map(|c| cell_of(c.trim()))
                        .collect::<Vec<TableCell>>()
                });
                let mut p = PivotTable::new(prop_list(node, "headers"), records);
                if let Some(by) = prop_str_opt(node, "pivot") {
                    let mut f = by.splitn(4, '|');
                    if let (Some(d), Some(a), Some(v)) = (f.next(), f.next(), f.next()) {
                        let how = match f.next().map(str::trim) {
                            Some("Average") => Aggregate::Average,
                            Some("Count") => Aggregate::Count,
                            Some("Min") => Aggregate::Min,
                            Some("Max") => Aggregate::Max,
                            _ => Aggregate::Sum,
                        };
                        p = p.pivot(d.trim(), a.trim(), v.trim(), how);
                    }
                }
                if let Some(d) = prop_num(node, "decimals") {
                    p = p.decimals(d as usize);
                }
                Some(p.into_any_element())
            }
            "TreeTable" => {
                fn tree_rows<'a>(
                    it: impl Iterator<Item = (&'a [String], &'a [String])>,
                ) -> Vec<TreeRow> {
                    let mut rows: Vec<TreeRow> = Vec::new();
                    let mut grouped: std::collections::BTreeMap<
                        String,
                        Vec<(&[String], &[String])>,
                    > = std::collections::BTreeMap::new();
                    for (path, cells) in it {
                        match path.split_first() {
                            Some((head, rest)) if rest.is_empty() => rows
                                .push(TreeRow::new(head.clone(), cells.iter().map(|c| cell_of(c)))),
                            Some((head, rest)) => {
                                grouped.entry(head.clone()).or_default().push((rest, cells))
                            }
                            None => {}
                        }
                    }
                    for (key, rests) in grouped {
                        rows.push(
                            TreeRow::new(key, Vec::new()).children(tree_rows(rests.into_iter())),
                        );
                    }
                    rows
                }
                let entries: Vec<(Vec<String>, Vec<String>)> = prop_list(node, "rows")
                    .iter()
                    .map(|r| {
                        let mut p = r.splitn(2, '|');
                        let path: Vec<String> = p
                            .next()
                            .unwrap_or("")
                            .split('>')
                            .map(|s| s.trim().to_owned())
                            .filter(|s| !s.is_empty())
                            .collect();
                        let cells: Vec<String> = p
                            .next()
                            .unwrap_or("")
                            .split('|')
                            .map(|c| c.trim().to_owned())
                            .collect();
                        (path, cells)
                    })
                    .collect();
                let mut t = TreeTable::new(
                    key.to_string(),
                    prop_list(node, "columns").iter().map(|c| column_of(c)),
                    tree_rows(entries.iter().map(|(p, c)| (p.as_slice(), c.as_slice()))),
                );
                t = t.open(prop_list(node, "open"));
                Some(t.into_any_element())
            }
            "TableToolbar" => {
                let mut t = TableToolbar::new(key.to_string());
                if prop_bool(node, "search") {
                    t = t.search(&self.field(node, key, view, _window, cx));
                }
                if let Some(rules) = prop_num(node, "filters") {
                    if let Some(h) = handler(view, node, "filter", cx) {
                        t = t.filter(rules as usize, h);
                    }
                }
                let columns: Vec<(SharedString, SharedString)> = prop_list(node, "columns")
                    .iter()
                    .filter_map(|e| {
                        let mut p = e.splitn(2, '|');
                        Some((
                            SharedString::from(p.next()?.trim().to_owned()),
                            SharedString::from(p.next().unwrap_or("").trim().to_owned()),
                        ))
                    })
                    .collect();
                if !columns.is_empty() {
                    let hidden = prop_list(node, "hidden");
                    let change = change_str(view, node, "hidden", cx);
                    t = t.columns(columns, hidden, move |v, w, cx| {
                        if let Some(change) = &change {
                            change(
                                v.iter()
                                    .map(|s| s.to_string())
                                    .collect::<Vec<_>>()
                                    .join(","),
                                w,
                                cx,
                            )
                        }
                    });
                }
                if let Some(h) = change_str(view, node, "density", cx) {
                    t = t.density(density_of(&prop_str(node, "density")), move |d, w, cx| {
                        h(format!("{d:?}"), w, cx)
                    });
                }
                if let Some(h) = handler(view, node, "export", cx) {
                    t = t.export(h);
                }
                Some(t.into_any_element())
            }
            "Typewriter" => {
                let mut t = Typewriter::new(key.to_string(), prop_str(node, "text"));
                if let Some(sp) = prop_num(node, "speed") {
                    t = t.speed(sp as f32);
                }
                Some(t.into_any_element())
            }
            "GradientText" => {
                let mut t = GradientText::new(prop_str(node, "text"));
                let (from, to) = (prop_str(node, "from"), prop_str(node, "to"));
                if let (Ok(a), Ok(b)) = (parse_hex(&from), parse_hex(&to)) {
                    t = t.colors(a, b);
                }
                Some(t.into_any_element())
            }
            "ShimmerText" => {
                Some(ShimmerText::new(key.to_string(), prop_str(node, "text")).into_any_element())
            }
            "Emoji" => {
                let glyph = prop_str(node, "glyph");
                let emoji = prop_str_opt(node, "shortcode")
                    .and_then(|c| Emoji::shortcode(&c))
                    .unwrap_or_else(|| Emoji::new(glyph));
                Some(emoji.into_any_element())
            }
            "Ellipsis" => Some(Ellipsis::new(prop_str(node, "text")).into_any_element()),
            "MiddleEllipsis" => {
                Some(MiddleEllipsis::new(prop_str(node, "text")).into_any_element())
            }
            "EllipsisTooltip" => Some(
                EllipsisTooltip::new(key.to_string(), prop_str(node, "text")).into_any_element(),
            ),
            "Kbd" => Some(Kbd::new(&prop_str(node, "keys")).into_any_element()),
            "KbdCombo" => Some(KbdCombo::new(&prop_str(node, "keys")).into_any_element()),
            "Highlight" => {
                let ranges = prop_list(node, "ranges")
                    .iter()
                    .filter_map(|r| {
                        let mut p = r.splitn(2, '-');
                        Some(
                            p.next()?.trim().parse::<usize>().ok()?
                                ..p.next()?.trim().parse::<usize>().ok()?,
                        )
                    })
                    .collect();
                Some(Highlight::new(prop_str(node, "text"), ranges).into_any_element())
            }
            "Blockquote" => {
                let mut b = Blockquote::new();
                b.extend(
                    self.child_views(node, key, cx)
                        .into_iter()
                        .map(|v| v.into_any_element()),
                );
                Some(b.into_any_element())
            }
            "Link" => {
                let on_click = {
                    let h = handler(view, node, "click", cx);
                    move |_e: &ClickEvent, w: &mut Window, cx: &mut App| {
                        if let Some(h) = &h {
                            h(w, cx)
                        }
                    }
                };
                Some(
                    Link::new(key.to_string(), prop_str(node, "label"), on_click)
                        .into_any_element(),
                )
            }
            "ExternalLink" => Some(
                ExternalLink::new(
                    key.to_string(),
                    prop_str(node, "label"),
                    prop_str(node, "url"),
                )
                .into_any_element(),
            ),
            "AnimatedNumber" => {
                let mut n =
                    AnimatedNumber::new(key.to_string(), prop_num(node, "value").unwrap_or(0.0));
                if let Some(d) = prop_num(node, "decimals") {
                    n = n.decimals(d as usize);
                }
                if let Some(p) = prop_num(node, "pad") {
                    n = n.pad(p as usize);
                }
                if prop_bool(node, "count_up") {
                    n = n.count_up();
                }
                Some(n.into_any_element())
            }
            "SelectableText" => Some(
                SelectableText::new(key.to_string(), prop_str(node, "text")).into_any_element(),
            ),
            "CurrencyText" => Some(
                CurrencyText::new(
                    prop_num(node, "amount").unwrap_or(0.0),
                    if prop_str(node, "code").is_empty() {
                        "USD"
                    } else {
                        match prop_str(node, "code").as_str() {
                            "EUR" => "EUR",
                            "GBP" => "GBP",
                            "JPY" => "JPY",
                            _ => "USD",
                        }
                    },
                )
                .into_any_element(),
            ),
            "FileSizeText" => Some(
                FileSizeText::new(prop_num(node, "bytes").unwrap_or(0.0).max(0.0) as u64)
                    .into_any_element(),
            ),
            "DurationText" => Some(
                DurationText::new(
                    prop_num(node, "seconds").unwrap_or(0.0).max(0.0) as u64,
                    match prop_str(node, "style").as_str() {
                        "Clock" => DurationStyle::Clock,
                        _ => DurationStyle::Compact,
                    },
                )
                .into_any_element(),
            ),
            "PluralText" => Some(
                PluralText::new(
                    prop_num(node, "count").unwrap_or(0.0).max(0.0) as u64,
                    "item",
                    "items",
                )
                .into_any_element(),
            ),
            "RelativeTime" => {
                let at = prop_str(node, "at")
                    .parse::<Timestamp>()
                    .unwrap_or_else(|_| Timestamp::now());
                Some(RelativeTime::new(key.to_string(), at).into_any_element())
            }
            "DateTimeText" => {
                let at = prop_str(node, "at")
                    .parse::<Timestamp>()
                    .unwrap_or_else(|_| Timestamp::now());
                Some(DateTimeText::new(at).into_any_element())
            }
            _ => None,
        }
    }
}

fn choices(node: &Node) -> Vec<Choice> {
    match prop(node, "options") {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::String(s) | Value::Enum(s) => Some(Choice::new(s.clone(), s.clone())),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn prop<'a>(node: &'a Node, name: &str) -> Option<&'a Value> {
    node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

fn prop_str_opt(node: &Node, name: &str) -> Option<String> {
    match prop(node, name)? {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        _ => None,
    }
}

fn prop_str(node: &Node, name: &str) -> String {
    node.content
        .as_ref()
        .and_then(|v| match v {
            Value::String(s) | Value::Enum(s) => Some(s.clone()),
            _ => None,
        })
        .or_else(|| prop_str_opt(node, name))
        .unwrap_or_default()
}

fn prop_bool(node: &Node, name: &str) -> bool {
    matches!(prop(node, name), Some(Value::Bool(true)))
}

fn prop_num(node: &Node, name: &str) -> Option<f64> {
    match prop(node, name)? {
        Value::Number { value, .. } => Some(*value),
        _ => None,
    }
}

fn tone_of(node: &Node) -> Tone {
    match prop_str_opt(node, "tone").as_deref() {
        Some("Accent") | Some("accent") => Tone::Accent,
        Some("Info") | Some("info") => Tone::Info,
        Some("Success") | Some("success") => Tone::Success,
        Some("Warning") | Some("warning") => Tone::Warning,
        Some("Danger") | Some("danger") => Tone::Danger,
        _ => Tone::Neutral,
    }
}

/// Build an Ely `Notification` row from a node's props.
fn notification_of(node: &Node, key: &str) -> Notification {
    let mut n = Notification::new(key.to_string(), icon_of(node), prop_str(node, "title"));
    if let Some(b) = prop_str_opt(node, "body") {
        n = n.body(b);
    }
    if let Some(t) = prop_str_opt(node, "time") {
        n = n.time(t);
    }
    if prop_bool(node, "unread") {
        n = n.unread(true);
    }
    n
}

fn text_size(node: &Node) -> TextSize {
    match prop_str_opt(node, "size").as_deref() {
        Some("Xs") | Some("xs") => TextSize::Xs,
        Some("Sm") | Some("sm") => TextSize::Sm,
        Some("Md") | Some("md") => TextSize::Md,
        Some("Lg") | Some("lg") => TextSize::Lg,
        Some("Xl") | Some("xl") => TextSize::Xl,
        Some("Xxl") | Some("xxl") => TextSize::Xxl,
        Some("Display") | Some("display") => TextSize::Display,
        _ => TextSize::Base,
    }
}

fn avatar_size(node: &Node) -> AvatarSize {
    match prop_str_opt(node, "size").as_deref() {
        Some("xs") | Some("Xs") => AvatarSize::Xs,
        Some("sm") | Some("Sm") => AvatarSize::Sm,
        Some("lg") | Some("Lg") => AvatarSize::Lg,
        Some("xl") | Some("Xl") => AvatarSize::Xl,
        _ => AvatarSize::Md,
    }
}

/// Build an Ely `Button` from a node's props (shared by `Button` and
/// `ButtonGroup` children).
fn button_of(node: &Node, key: &str) -> Button {
    let mut b = Button::new(key.to_string(), prop_str(node, "label"));
    if let Some(v) = prop_str_opt(node, "variant") {
        b = b.variant(match v.as_str() {
            "Secondary" => ButtonVariant::Secondary,
            "Outline" => ButtonVariant::Outline,
            "Ghost" => ButtonVariant::Ghost,
            "Subtle" => ButtonVariant::Subtle,
            "Success" => ButtonVariant::Success,
            "Danger" => ButtonVariant::Danger,
            "Link" => ButtonVariant::Link,
            _ => ButtonVariant::Primary,
        });
    }
    if let Some(s) = prop_str_opt(node, "size") {
        b = b.size(match s.as_str() {
            "Sm" => ControlSize::Sm,
            "Lg" => ControlSize::Lg,
            _ => ControlSize::Md,
        });
    }
    if let Some(i) = prop_str_opt(node, "icon").and_then(|n| icon_named(&n)) {
        b = b.icon(i);
    }
    if prop_bool(node, "disabled") {
        b = b.disabled(true);
    }
    b
}

fn severity_of(node: &Node) -> Severity {
    match prop_str_opt(node, "severity").as_deref() {
        Some("Success") => Severity::Success,
        Some("Warning") => Severity::Warning,
        Some("Danger") => Severity::Danger,
        _ => Severity::Info,
    }
}

fn prop_list(node: &Node, name: &str) -> Vec<String> {
    match prop(node, name) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::String(s) | Value::Enum(s) => Some(s.clone()),
                Value::Number { value, .. } => Some(value.to_string()),
                _ => None,
            })
            .collect(),
        Some(Value::String(s)) | Some(Value::Enum(s)) => s
            .split(',')
            .map(|x| x.trim().to_owned())
            .filter(|x| !x.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn prop_floats(node: &Node, name: &str) -> Vec<f32> {
    prop_list(node, name)
        .iter()
        .filter_map(|s| s.parse::<f32>().ok())
        .collect()
}
