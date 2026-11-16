# Supported components

> Generated from [ALL-COMPONENTS.md](https://github.com/skamansam/gpui-forge/blob/main/ALL-COMPONENTS.md) by `site/scripts/generate-components.mjs` — do not edit by hand.

Every component set Forge understands, what each provides, and how to enable it. A document picks a rendering base with `base:` and pulls adapter libraries in through `use:` imports — see [adapters](adapters.md).

**746 supported elements** across 20 sets. 270 upstream exports were reviewed and deliberately not exposed — each carries a reason below (value types, traits, engine handles, or runtime requirements a declarative document cannot express).

| Set | Enable | Supported | Reviewed — not widgets |
| --- | --- | --- | --- |
| [GPUI-KIT Components](#gpui-kit-components) | `base: gpui-kit` (default) | 73 | 40 |
| [GPUI-KIT sub-elements (catalog)](#gpui-kit-sub-elements-catalog) | inside gpui-kit parents | 59 | 0 |
| [ely Components](#ely-components) | `use: ely: ely-core` | 302 | 89 |
| [guise Components](#guise-components) | `base: guise` | 125 | 32 |
| [gpui-flow Components](#gpui-flow-components) | `use: gflow: gpui-flow` | 1 | 0 |
| [ferrum-flow Components](#ferrum-flow-components) | `use: flow: ferrum-flow` | 1 | 14 |
| [base-gpui Components](#base-gpui-components) | `use: base: base-gpui` | 10 | 0 |
| [video Components](#video-components) | `use: video: gpui-video-player` | 1 | 2 |
| [apple-hig Components](#apple-hig-components) | `use: hig: gpui-apple-hig` | 28 | 20 |
| [pdf Components](#pdf-components) | `use: pdf: gpui-pdf` | 1 | 6 |
| [whiteboard Components](#whiteboard-components) | `use: wb: gpui-whiteboard` | 1 | 0 |
| [ratex Components](#ratex-components) | `use: math: ratex-gpui` | 2 | 2 |
| [rhythm Components](#rhythm-components) | `use: rhythm: rhythm-gpui` | 1 | 3 |
| [gpui-symbols Components](#gpui-symbols-components) | `use: symbols: gpui-symbols` | 1 | 3 |
| [ratatui Components](#ratatui-components) | `base: ratatui` | 14 | 13 |
| [iced Components](#iced-components) | `base: iced` | 28 | 7 |
| [bezel Components](#bezel-components) | `base: bezel` (scaffold) | 6 | 0 |
| [adabraka Components](#adabraka-components) | catalog only (scaffold) | 84 | 39 |
| [forge built-ins](#forge-built-ins) | always available | 7 | 0 |
| [template-lib Components](#template-lib-components) | adapter scaffold | 1 | 0 |

## GPUI-KIT Components

[docs](https://gpui-kit.com/) · [github](https://github.com/longbridge/gpui-kit)

> Remaining exports of `gpui_kit::component` (the upstream `gpui-component` crate re-exported through gpui-kit — same surface, not a separate set) reviewed and deliberately not cataloged:

<details><summary><strong>73 supported elements</strong></summary>

[`Accordion`](https://gpui.rs/components/accordion) [`AlertDialog`](https://gpui.rs/components/alertdialog) [`Alert`](https://gpui.rs/components/alert) [`Attachment`](https://gpui.rs/components/attachment) [`Avatar`](https://gpui.rs/components/avatar) [`Badge`](https://gpui.rs/components/badge) [`Bubble`](https://gpui.rs/components/bubble) [`Button`](https://gpui.rs/components/button) [`Calendar`](https://gpui.rs/components/calendar) [`Carousel`](https://gpui.rs/components/carousel) [`Chart`](https://gpui.rs/components/chart) (LineChart, BarChart, PieChart) [`Checkbox`](https://gpui.rs/components/checkbox) [`Clipboard`](https://gpui.rs/components/clipboard) [`Collapsible`](https://gpui.rs/components/collapsible) [`ColorPicker`](https://gpui.rs/components/colorpicker) [`Combobox`](https://gpui.rs/components/combobox) [`Command`](https://gpui.rs/components/command) [`DataTable`](https://gpui.rs/components/datatable) (alias of Table) [`DatePicker`](https://gpui.rs/components/datepicker) [`DescriptionList`](https://gpui.rs/components/descriptionlist) [`Dialog`](https://gpui.rs/components/dialog) [`Diff`](https://gpui.rs/components/diff) [`DropdownButton`](https://gpui.rs/components/dropdownbutton) [`Editor`](https://gpui.rs/components/editor) [`Empty`](https://gpui.rs/components/empty) [`Form`](https://gpui.rs/components/form) [`GridLayout`](https://gpui.rs/components/gridlayout) [`GroupBox`](https://gpui.rs/components/groupbox) [`HoverCard`](https://gpui.rs/components/hovercard) [`Icon`](https://gpui.rs/components/icon) [`Image`](https://gpui.rs/components/image) [`Input`](https://gpui.rs/components/input) [`Kbd`](https://gpui.rs/components/kbd) [`Label`](https://gpui.rs/components/label) [`List`](https://gpui.rs/components/list) [`Marker`](https://gpui.rs/components/marker) [`Message`](https://gpui.rs/components/message) [`Notification`](https://gpui.rs/components/notification) [`NumberInput`](https://gpui.rs/components/numberinput) [`OtpInput`](https://gpui.rs/components/otpinput) [`Pagination`](https://gpui.rs/components/pagination) [`Popover`](https://gpui.rs/components/popover) [`Progress`](https://gpui.rs/components/progress) [`Questionnaire`](https://gpui.rs/components/questionnaire) [`Radio`](https://gpui.rs/components/radio) [`Rating`](https://gpui.rs/components/rating) [`Resizable`](https://gpui.rs/components/resizable) [`Scrollable`](https://gpui.rs/components/scrollable) [`Select`](https://gpui.rs/components/select) [`Settings`](https://gpui.rs/components/settings) [`Sheet`](https://gpui.rs/components/sheet) (via `Dialog#id` + `sheet: true`) [`Shimmer`](https://gpui.rs/components/shimmer) [`Sidebar`](https://gpui.rs/components/sidebar) [`Skeleton`](https://gpui.rs/components/skeleton) [`Slider`](https://gpui.rs/components/slider) [`Spinner`](https://gpui.rs/components/spinner) [`StatusBar`](https://gpui.rs/components/statusbar) [`Stepper`](https://gpui.rs/components/stepper) [`Switch`](https://gpui.rs/components/switch) [`Table`](https://gpui.rs/components/table) [`Tabs`](https://gpui.rs/components/tabs) [`Tag`](https://gpui.rs/components/tag) [`TextView`](https://gpui.rs/components/textview) [`Textarea`](https://gpui.rs/components/textarea) [`TimeField`](https://gpui.rs/components/timefield) [`TitleBar`](https://gpui.rs/components/titlebar) [`Toggle`](https://gpui.rs/components/toggle) (alias of Switch) [`Toolbar`](https://gpui.rs/components/toolbar) [`Tree`](https://gpui.rs/components/tree) [`VirtualList`](https://gpui.rs/components/virtuallist) (alias of List) `RadarChart` (implemented (base arm, `data` series)) `SankeyChart` (implemented (`nodes`/`links` props)) `WindowBorder` (implemented (child wrapper))

</details>

<details><summary><strong>40 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `Dock` | needs a panel registry |
| `Menu` | native app menus are host-level; in-doc menus: ContextMenu/DropdownMenu |
| `MessageScroller` | needs a per-row render callback |
| `Plot` | chart internals — LineChart/BarChart/PieChart cover rendering |
| `Speech` | hardware speech recognizer |
| `Theme` | window-level theme — handled by the app shell |
| `Tooltip` | attribute-level (`.tooltip()` on any element) — no standalone element |
| `Cancel` | gpui `Action` — dialog cancel, not a widget |
| `Confirm` | gpui `Action` — dialog confirm, not a widget |
| `Date` | value type, not a widget — `DatePicker`/`Calendar` produce it |
| `DiffConflict` | merge-conflict value type for the diff viewer |
| `DiffConflictResolution` | enum — which side resolves a conflict |
| `DiffFile` | diff document value type behind DiffView |
| `DiffFileStatus` | enum — added/modified/deleted for DiffFile |
| `DiffSide` | enum — old/new side of a diff hunk |
| `Disableable` | trait — `.disabled(bool)` support |
| `History` | undo/redo entity behind inputs |
| `HourCycle` | enum — 12h/24h for TimeField/DatePicker |
| `Matcher` | fuzzy-match trait behind SearchableList |
| `NumberStep` | enum — NumberInput step behavior |
| `PlotHover` | chart hover value type behind interactive charts |
| `PopupMenu` | menu entity — `PopupMenu::build(cx)` attaches to a trigger, not a standalone element |
| `PopupMenuItem` | menu item descriptor consumed by PopupMenu |
| `RadarLabel` | axis label value type for RadarChart |
| `RoleOverride` | a11y role-override trait |
| `Rope` | rope text buffer behind Editor/inputs |
| `RopeLines` | rope line iterator behind Editor |
| `SankeyLabel` | edge label descriptor — SankeyChart builds it |
| `SearchableGroup` | grouped-list section type for SearchableList |
| `SearchableListDelegate` | trait — supplies SearchableList rows |
| `SearchableListItem` | list row value type for SearchableList |
| `SearchableVec` | searchable row container for SearchableList |
| `Selectable` | trait — `.selected(bool)` support |
| `Sizable` | trait — `.size(...)` support |
| `SliderScale` | enum — linear/log slider scale |
| `SpeechStatus` | enum — speech-button state (recording/processing) |
| `StyleSized` | trait — style-driven sizing |
| `TimePrecision` | enum — TimeField granularity |
| `TimeSegment` | enum — hour/minute/second segment of TimeField |
| `UndoHistory` | undo stack entity behind inputs |

</details>

## GPUI-KIT sub-elements (catalog)

[docs](https://gpui-kit.com/) · [github](https://github.com/longbridge/gpui-kit)

> Child elements — render inside their parent arms.

<details><summary><strong>59 supported elements</strong></summary>

`Radio` `Tab` `MenuItem` `MenuSeparator` `BreadcrumbItem` `DescriptionItem` `StepperItem` `AccordionItem` `MessageAvatar` `MessageHeader` `MessageContent` `MessageFooter` `TableHeader` `TableBody` `TableFooter` `TableCaption` `TableRow` `ResizablePanel` `SidebarHeader` `SidebarFooter` `SidebarMenu` `SidebarMenuItem` `TreeItem` `CommandItem` `CommandGroup` `CommandSeparator` `QuestionnaireItem` `QuestionnaireChoice` `Sheet` `Box` `VStack` `HStack` `SettingPage` `SettingsGroup` `SettingItem` `Text` `RadioGroup` `Separator` `DropdownMenu` `Link` `Breadcrumb` `ProgressCircle` `AvatarGroup` `FormField` `TableHead` `TableCell` `Grid` `FocusTrap` `ContextMenu` `InputGroup` `InputGroupInput` `InputGroupTextarea` `InputGroupAddon` `InputGroupText` `InputGroupButton` `LineChart` `BarChart` `PieChart` `SwitchButton`

</details>


## ely Components

[docs](https://elygpui.com/) · [github](https://github.com/ELY-Open-Source/Ely-GPUI-Components)

> The ely site advertises 1279 items (widgets + ~900 icons). Below is every exported widget type in `ely-core` 0.2 — the `[x]` entries render natively via `forge-lib-ely` (`use: ely: ely-core`); the rest have no Forge adapter support yet.

<details><summary><strong>302 supported elements</strong></summary>

`Accordion` `AccordionItem` `ActionBar` `Activity` `ActivityBar` `ActivityFeed` `Alert` `AlertDialog` `Anchor` `AnimatePresence` `AnimatedGradient` `AppShell` `AspectRatio` `Avatar` `AvatarGroup` `Backdrop` `Badge` `Banner` `Barcode` `BeforeAfter` `Blink` `Blockquote` `Breadcrumb` `BulkActionBar` `Button` `ButtonGroup` `Calendar` `Callout` `Caption` `Card` `CardHeader` `Carousel` `Cascader` `Changelog` `Checkbox` `CheckboxCard` `CheckboxGroup` `Code` `Collapsible` `ColorSwatch` `ColumnShell` `Comparison` `Confetti` `ConfirmButton` `ConfirmDialog` `ConnectionStatus` `Container` `ContextMenu` `CountBadge` `Countdown` `DataGrid` `DataTable` `DatePicker` `DateRangePicker` `DateTimePicker` `DescriptionList` `Dialog` `DirectoryListing` `Dock` `DockPanel` `DockSide` `DotBadge` `Drawer` `DropZone` `DropdownMenu` `DurationPicker` `EditorTabs` `Ellipsis` `EllipsisTooltip` `EmailInput` `EmojiPicker` `ErrorBoundary` `ErrorView` `ExpressionInput` `ExternalLink` `FieldArray` `Fieldset` `FileInput` `FileTree` `Flash` `Flip` `FloatingToolbar` `FontPicker` `Frame` `Gallery` `Gauge` `Glow` `GradientEditor` `GradientText` `Heading` `HeatmapTable` `HelpTooltip` `Highlight` `IconButton` `IconPicker` `Image` `InfiniteList` `InlineEdit` `InlineMessage` `Input` `InputAddon` `InputGroup` `IntersectionObserver` `Kbd` `KbdCombo` `KeyValueInput` `KpiCard` `Label` `LazyLoad` `Lightbox` `Link` `List` `ListInput` `ListItem` `LoadMore` `LoadingOverlay` `Marquee` `MaskedInput` `Masonry` `MasterDetail` `Measure` `Meter` `MiddleEllipsis` `MonthPicker` `NavGroup` `NavItem` `Notification` `NotificationCenter` `NumberInput` `OfflineIndicator` `OverflowMenu` `Overline` `Page` `Pagination` `PaneGroup` `PaneId` `PaneLayout` `Panel` `Paragraph` `ParticleBackground` `PasswordInput` `PathInput` `Peek` `PhoneInput` `PieMenu` `ProgressBar` `ProgressRing` `PromptDialog` `PropertyGrid` `PropertyGroup` `Pulse` `QrCode` `QuarterPicker` `QuickOpen` `QuickSwitcher` `Radio` `RadioCard` `RadioGroup` `RangeSlider` `RefreshIndicator` `RegexInput` `RelativeDatePicker` `Release` `Ripple` `SavingIndicator` `ScrollShadow` `ScrollToTop` `ScrubInput` `SearchableMenu` `Section` `Sections` `SelectableList` `Shake` `Sheet` `ShellColumn` `Shimmer` `ShimmerText` `SignaturePad` `SimpleGrid` `Skeleton` `SkeletonAvatar` `SkeletonCard` `SkeletonTable` `SkeletonText` `Slider` `Sparkline` `Spinner` `SplitButton` `SplitPane` `Spreadsheet` `Stagger` `Stars` `Statistic` `StatusBar` `StatusBarItem` `StatusMessage` `Steps` `StickyHeader` `Subtitle` `Suspense` `SwipeableListItem` `SyncStatus` `TabBar` `Table` `TableOfContents` `Tabs` `Tag` `TimePicker` `Timeline` `TimelineItem` `Title` `TitleBar` `Toast` `ToastViewport` `Toaster` `ToggleButton` `ToggleGroup` `ToggleItem` `Toolbar` `ToolbarGroup` `ToolbarSeparator` `Transform` `Tree` `TreeSelect` `TreeTable` `TrendIndicator` `Typewriter` `TypingIndicator` `UpdateBanner` `UpdateDialog` `UploadList` `UrlInput` `UsageBar` `UserChip` `Viewport` `VirtualList` `Watermark` `Well` `WindowControls` `Wizard` `YearPicker` `ZStack` `ZoomControl` `Divider` `CopyButton` `SegmentedControl` `EmptyState` `Switch` `Rating` `Stepper` `Icon` `AvatarUpload` `CodeInput` `ColorPicker` `Combobox` `CronEditor` `DirtyIndicator` `EyeDropper` `FormError` `FormField` `FormLabel` `FormSection` `InlineForm` `JsonInput` `ListBox` `MentionInput` `SearchInput` `TagInput` `TimezoneSelect` `TransferList` `UnitInput` `WeekPicker` `Outline` `SortableList` `GroupedList` `BackForwardNavigation` `GoToLine` `NavigationMenu` `TabOverflowMenu` `CrashReporter` `HoverCard` `Popover` `ResizeBorder` `Vibrancy` `WindowSwitcher` `AnimatedNumber` `CurrencyText` `DateTimeText` `DurationText` `Emoji` `FileSizeText` `Link` `PivotTable` `PluralText` `RelativeTime` `SelectableText` `TableToolbar` `ProgressBar` `Reorder` `Transition` `TooltipTrigger` `CommandPalette`

</details>

<details><summary><strong>89 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `AsyncView` | needs a `Future` + `Context` — async entity, not declarative |
| `COUNTRIES` | `(&str,&str,&str,&str)` constant table — PhoneInput country list |
| `Cascade` | Cascader option node — built from `options` props |
| `Change` | change-record value type behind inputs |
| `Changeable` | trait — widgets emit Change events |
| `Close` | `Rc<dyn Fn>` dismiss-callback alias |
| `Column` | column descriptor — tables build it from `columns` props |
| `Command` | palette item descriptor — `value`/`label`/`keys` consumed by CommandPalette `groups` |
| `Connectivity` | ConnectionStatus state enum — set via `state` prop |
| `Copy` | gpui `Action` — copy command, not a widget |
| `Crumb` | Breadcrumb item descriptor — built from `items` props |
| `Cut` | gpui `Action` — cut command, not a widget |
| `Disableable` | trait — `disabled` prop support |
| `DockLayout` | layout model Dock owns internally |
| `Drawing` | internal pen-stroke entity behind SignaturePad |
| `DropAt` | drop-position enum for drag targets |
| `Dropdown` | `#[doc(hidden)]` picker popup state, owned by date pickers |
| `Editing` | editor-state enum behind InlineEdit |
| `EditorTab` | EditorTabs tab descriptor — built from `items` props |
| `Entrance` | Transition/Stagger `entrance` enum (Fade/Rise/Slide) |
| `Finder` | internal glyph-finder machinery of EmojiPicker/IconPicker |
| `Folder` | `#[doc(hidden)]` tree builder backing FileTree |
| `Grid` | internal glyph grid of EmojiPicker/IconPicker |
| `HUE_NAMES` | hue-name constant table for color pickers |
| `History` | undo/redo history entity behind text inputs |
| `LOCALES` | locale constant table |
| `Menu` | menu model — DropdownMenu/ContextMenu own it via OwnedMenu |
| `MenuItem` | menu item descriptor consumed by menu `items` props |
| `Mix` | gradient color-stop pair for GradientEditor |
| `OnClimb` | `Rc<dyn Fn>` breadcrumb-climb callback alias |
| `OnFold` | `Rc<dyn Fn>` section-fold callback alias |
| `PICTURES` | sample-picture constant table for demos |
| `Paste` | gpui `Action` — paste command, not a widget |
| `Pick` | selection enum + `Rc` pick-callback alias |
| `PieItem` | PieMenu sector descriptor |
| `Place` | positioning enum (Center/Left/Right/Top/Bottom) |
| `Presence` | Avatar presence enum — set via `presence` prop |
| `Raised` | elevation/style enum |
| `Refresh` | RefreshIndicator `state` enum (Idle/Pulling/Refreshing/Done) |
| `ScrollModel` | virtual-scroll state model behind VirtualList/InfiniteList |
| `ScrollPlace` | scroll-position value type |
| `SelectAll` | gpui `Action` — select-all command, not a widget |
| `Selectable` | trait — selection support |
| `Sizable` | trait — `size` prop support |
| `Spot` | Spotlight/Tour target descriptor needing live Bounds |
| `Stroke` | `Vec<Point<Pixels>>` alias — signature ink data |
| `Syntax` | static language-rule table behind CodeInput (`syntax_of`) |
| `TabPlacement` | Tabs placement enum (Top/Bottom/Left/Right) |
| `TextInput` | input state entity; Forge inputs manage it internally |
| `Tone` | severity/color enum — set via `tone` props |
| `Tooltip` | content factory (`Tooltip::text`) for TooltipTrigger, not a widget |
| `TreeNode` | tree node value type — built from `nodes` props |
| `Upload` | row value type consumed by UploadList `uploads` |
| `WORKSPACE_VERSION` | workspace-schema version constant |
| `Workspace` | dock capture/apply session JSON — `to_json`/`apply`, not a widget |
| `ZoomStep` | zoom-level value type behind ZoomControls |
| `MenuBar` | menu items need a boxed gpui `Action`, not declarative handlers |
| `Tour` | TourStep needs live `Bounds<Pixels>` of a target element |
| `Spotlight` | same: step target is a `Bounds<Pixels>`, no doc-level geometry |
| `TourStep` | value type holding a `Bounds<Pixels>` target |
| `WindowTab` | descriptor consumed by TabBar, not standalone |
| `ActivityItem` | descriptor consumed by ActivityBar children |
| `WindowManager` | app-level window host registry, not a widget |
| `Hosted` | wrapper type for `open`-spawned window content |
| `MiniWindow` | spawns a real hosted window, not in-tree render |
| `Corner` | window-corner value type |
| `AboutDialog` | `.open()` spawns a hosted window, not an element |
| `SplashScreen` | `.open()` spawns a hosted window, not an element |
| `Sections` | scroll-state entity backing TOC/Anchor, not a widget |
| `media_button` | helper fn, not a component |
| `menu_under` | helper fn, not a component |
| `guard_close` | window-lifecycle helper fn, not a component |
| `FilterBuilder` | interactive rule editor needing owner callbacks |
| `SortBuilder` | interactive sort editor needing owner callbacks |
| `FilterRule` | rule value type consumed by DataTable.filters |
| `SortKey` | sort value type consumed by DataTable.sorts |
| `Test` | filter-operator enum, not a widget |
| `to_csv` | helper fn, not a component |
| `Aggregate` | cell-aggregation enum, not a widget |
| `Align` | column-align enum, not a widget |
| `Cell` | table cell value type (encoded in `rows` props) |
| `Row` | table row value type (built from `rows` props) |
| `TreeRow` | tree row value type (built from `rows` props) |
| `text_width` | helper fn, not a component |
| `literal` | helper fn, not a component |
| `tabular` | helper fn, not a component |
| `fresh` | helper fn, not a component |
| `relative_text` | helper fn, not a component |
| `LEADING` | typography constant |

</details>

## guise Components

[docs](https://docs.rs/guise) · [github](https://github.com/wess/guise)

> Vanilla-gpui flavor base — `base: guise` on crates.io `gpui 0.2.2`. Below is every exported widget type in `guise-ui` 1.10 — `[x]` entries are mapped by `catalogs/guise.catalog.yaml` (`Stack`/`Group` surface as Forge `VStack`/`HStack`); each unchecked entry carries a one-line reason it stays unmapped.

<details><summary><strong>125 supported elements</strong></summary>

`About` `Accordion` `ActionIcon` `AIChatView` `AICitation` `AIComposer` `AICost` `AIMessage` `AIModelPicker` `AIReasoning` `AISettings` `AISources` `AIStreamingText` `AIThinking` `AITokenMeter` `AIToolCall` `Alert` `Anchor` `AppShell` `Autocomplete` `Avatar` `AvatarGroup` `Badge` `BarChart` `Blockquote` `Breadcrumbs` `Button` `Calendar` `Card` `Carousel` `Center` `Checkbox` `CheckboxGroup` `Chip` `CloseButton` `Code` `ColorInput` `Combobox` `ConfirmModal` `Container` `ContextMenu` `CopyButton` `DataView` `DatePicker` `Divider` `Drawer` `Dropzone` `Editor` `Field` `FileInput` `Group` `HoverCard` `Icon` `Image` `Indicator` `Kbd` `LineChart` `List` `Loader` `LoadingOverlay` `Markdown` `MarkdownEditor` `Mark` `Menu` `MenuBar` `Modal` `NavigationMenu` `NavLink` `Notification` `NumberInput` `Pagination` `Panel` `Paper` `PasswordInput` `PieChart` `PinInput` `Popover` `Progress` `Radio` `RadioGroup` `RangeSlider` `Rating` `RingProgress` `ScatterChart` `ScrollArea` `SegmentedControl` `Select` `SettingsRow` `SettingsSection` `SettingsView` `SimpleGrid` `Skeleton` `Slider` `SortableList` `Space` `Sparkline` `SplitPanel` `Spoiler` `Spotlight` `Stack` `StatusBar` `Stepper` `Switch` `TabBar` `Table` `TableView` `Tabs` `TagsInput` `Text` `TextArea` `TextInput` `ThemeIcon` `ThemePicker` `TimePicker` `Timeline` `Title` `ToastStack` `Tooltip` `Tour` `Transfer` `TreeView` `VideoView` `VirtualList` `WebView` `WindowControls`

</details>

<details><summary><strong>32 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `AIModel` | model descriptor consumed by `AIModelPicker`, not a widget |
| `AIPricing` | pricing descriptor consumed by `AICost`, not a widget |
| `AISource` | source descriptor consumed by `AISources`/`AIChatView`, not a widget |
| `AITurn` | turn descriptor consumed by `AIChatView`, not a widget |
| `AITurnTool` | tool descriptor consumed by `AITurn`, not a widget |
| `AIUsage` | token-usage descriptor consumed by `AICost`, not a widget |
| `Animated` | animation primitive — wraps an element tree, not a standalone widget |
| `Animator` | drives `Animated` — construct it from Forge state instead |
| `Collapse` | animation primitive — use `Spoiler` for expandable content |
| `Date` | value type, not a widget — `DatePicker`/`Calendar` produce it |
| `DevTools` | in-app debug panel — developer tooling, not a document element |
| `Draggable` | drag-source primitive — `SortableList` covers reorderable lists |
| `DropTarget` | drop-target primitive — needs host drag state; `Dropzone` covers file drops |
| `GpuView` | raw wgpu surface — needs a caller-supplied `GpuScene` render callback |
| `MenuColumn` | internal layout primitive of `Menu`/`MenuBar` |
| `Motioned` | animation wrapper — applies motion curves to a child element |
| `OverlayHost` | internal mounting point — overlays host themselves |
| `PaneGroup` | unconstructable — its `ItemId` is `pub(crate)` upstream |
| `Presence` | exit-animation primitive — wraps elements, not a standalone widget |
| `ResizeHandles` | internal part of `SplitPanel` |
| `Responsive` | generic viewport switch — takes a layout closure, not props |
| `Scrollbar` | internal part of `ScrollArea` |
| `Sequence` | animation sequencing primitive |
| `SettingsPage` | page descriptor consumed by `SettingsView`, not a widget |
| `Stagger` | animation sequencing primitive |
| `TextEdit` | editor value type (cursor/undo model), not a widget |
| `Time` | value type, not a widget — `TimePicker` produces it |
| `Transition` | animation primitive |
| `UpdateNotice` | needs a live `Updater`/`UpdateOutcome` — host update infrastructure |
| `UpdatePrompt` | needs a live `Updater`/`Release` — host update infrastructure |
| `VideoFeed` | frame source consumed by `VideoView`, not a standalone widget |
| `VideoFrame` | single frame consumed by `VideoView`, not a widget |

</details>

## gpui-flow Components

[docs](https://github.com/pacifio/gpui-flow) · [github](https://github.com/pacifio/gpui-flow)

> `use: gflow: gpui-flow` — node-graph canvas, rendered by `forge-lib-flow`.

<details><summary><strong>1 supported elements</strong></summary>

`Canvas`

</details>


## ferrum-flow Components

[docs](https://crates.io/crates/ferrum-flow) · [github](https://crates.io/crates/ferrum-flow)

> `use: flow: ferrum-flow` (guise feature-gated adapter).

<details><summary><strong>1 supported elements</strong></summary>

`Canvas`

</details>

<details><summary><strong>14 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `ActiveNodeDrag` | in-flight drag state behind Flow's node dragging |
| `Command` | undo/redo command trait — app supplies implementations |
| `CompositeCommand` | batch of `Command`s — internal undo machinery |
| `ContextMenuCanvasExtra` | context-menu payload for Flow's canvas |
| `CreateEdge` | command/descriptor used to insert edges |
| `CreatePort` | command/descriptor used to insert ports |
| `GraphChange` | change record in the graph history log |
| `GraphOp` | enum — graph mutation ops behind `Flow` |
| `HistoryProvider` | trait — pluggable undo backends for Flow |
| `LocalHistory` | default in-memory `HistoryProvider` impl |
| `NODE_DRAG_TICK_INTERVAL` | const — drag-update cadence |
| `NodeRenderer` | trait — app-supplied node body renderer for Flow |
| `ToastLevel` | enum — toast severity for Flow's notices |
| `ToastMessage` | notice value type for Flow's toast overlay |

</details>

## base-gpui Components

[docs](https://github.com/LukeTandjung/base-gpui) · [github](https://github.com/LukeTandjung/base-gpui)

> Vendored Zed `gpui` engine fork — `use: base: base-gpui`, rendered by `forge-lib-base`. The crate also exports the full gpui element/element-style API (engine primitives — not a widget set).

<details><summary><strong>10 supported elements</strong></summary>

`Button` `Checkbox` `Switch` `Toggle` `Slider` `Progress` `Meter` `Separator` `Input` `Avatar`

</details>


## video Components

[docs](https://github.com/cijiugechu/gpui-video-player) · [github](https://github.com/cijiugechu/gpui-video-player)

> `use: video: gpui-video-player` — GStreamer 1.14+ required.

<details><summary><strong>1 supported elements</strong></summary>

`Player`

</details>

<details><summary><strong>2 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `Video` | engine handle (Arc<RwLock<Internal>>) owning the GStreamer pipeline — not an element |
| `VideoElement` | needs a live `Video` handle + GStreamer runtime — can't be constructed declaratively |

</details>

## apple-hig Components

[docs](https://crates.io/crates/gpui-apple-hig) · [github](https://crates.io/crates/gpui-apple-hig)

> `use: hig: gpui-apple-hig` — rendered by `forge-lib-hig`. `[x]` = reachable as `hig.<note>` (forge element name).

<details><summary><strong>28 supported elements</strong></summary>

`MacActivityRings` (hig.ActivityRings) `MacBadge` (hig.Badge) `MacButton` (hig.Button) `MacChart` (hig.Chart) `MacCheckbox` (hig.Checkbox) `MacColumnView` (hig.ColumnView) `MacContextMenu` (hig.ContextMenu) `MacDatePicker` (hig.DatePicker) `MacDialog` (hig.Dialog) `MacGauge` (hig.Gauge) `MacImageView` (hig.ImageView) `MacKbd` (hig.Kbd) `MacMenuBar` (hig.MenuBar) `MacOutlineView` (hig.OutlineView) `MacPanel` (hig.Panel) `MacPathControl` (hig.PathControl) `MacPopover` (hig.Popover) `MacPopupButton` (hig.PopupButton) `MacProgress` (hig.Progress) `MacPullDownButton` (hig.PullDownButton) `MacSegmentedControl` (hig.SegmentedControl) `MacSettingsGroup` (hig.SettingsGroup) `MacSidebar` (hig.Sidebar) `MacSpinner` (hig.Spinner) `MacSwitch` (hig.Switch) `MacTabBar` (hig.TabBar) `MacTableView` (hig.TableView) `MacToolbar` (hig.Toolbar)

</details>

<details><summary><strong>20 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `AppleAppearance` | theme enum — light/dark, set on the window not an element |
| `AppleColors` | semantic color token table — theme values, not widgets |
| `MAC_TRAFFIC_LIGHT_CLEARANCE` | window-layout constant (px offset) |
| `MONO_FONT_NAME` | font-name constant |
| `MacAccentColor` | accent-color enum — set via `color`/`accent` props |
| `MacChartDataPoint` | chart point value type — built from `points` props |
| `MacColumnData` | column-view column value type — built from `columns` props |
| `MacMenuBarMenu` | menu name descriptor — built from `menus` props |
| `MacMenuItem` | menu item descriptor — built from `items` props |
| `MacOutlineItem` | outline row descriptor — built from `items` props |
| `MacPathItem` | path segment descriptor — built from `items` props |
| `MacRingMetric` | ring value type — built from `rings` props |
| `MacSegment` | segment descriptor — built from `items` props |
| `MacSettingsRow` | settings row descriptor — children of SettingsGroup build it |
| `MacSidebarItem` | sidebar row descriptor — built from `items`/children |
| `MacSidebarSection` | sidebar section descriptor — children group items |
| `MacTabItem` | tab descriptor — built from `items`/children props |
| `MacTableColumn` | column descriptor — built from `columns` props |
| `MacTableRow` | row value type — built from `rows` props |
| `SYSTEM_FONT_NAME` | font-name constant |

</details>

## pdf Components

[docs](https://packetthrower.github.io/zorite/reference/crates/gpui-pdf/) · [github](https://github.com/packetThrower/zorite)

> `use: pdf: gpui-pdf` — opt-in zorite adapter (`forge-lib-zorite` `pdf` feature).

<details><summary><strong>1 supported elements</strong></summary>

`Viewer`

</details>

<details><summary><strong>6 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `LinkTarget` | link destination value type for PdfView |
| `NormPoint` | normalized-coordinate point for hit testing |
| `NormRect` | normalized-coordinate rect for hit testing |
| `OutlineItem` | document-outline node consumed by PdfView |
| `PageText` | extracted page text value type |
| `PdfLink` | link annotation value type |

</details>

## whiteboard Components

[docs](https://packetthrower.github.io/zorite/) · [github](https://github.com/packetThrower/zorite)

> `use: wb: gpui-whiteboard` — opt-in zorite adapter.

<details><summary><strong>1 supported elements</strong></summary>

`Board`

</details>


## ratex Components

[docs](https://packetthrower.github.io/zorite/) · [github](https://github.com/packetThrower/zorite)

> `use: math: ratex-gpui` — TeX math rendering, opt-in zorite adapter.

<details><summary><strong>2 supported elements</strong></summary>

`Tex` `Editor`

</details>

<details><summary><strong>2 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `MathEditor` | `impl Render` entity — needs Entity<MathEditor> + keymap/editor setup |
| `MathNav` | enum — editor navigation ops |

</details>

## rhythm Components

[docs](https://crates.io/crates/rhythm-gpui) · [github](https://crates.io/crates/rhythm-gpui)

> `use: rhythm: rhythm-gpui` (guise feature-gated adapter).

<details><summary><strong>1 supported elements</strong></summary>

`Frame`

</details>

<details><summary><strong>3 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `DropCapRhythm` | typography metric struct, not a widget |
| `FontRhythm` | typography metric struct, not a widget |
| `Rhythm` | typography metric struct, not a widget |

</details>

## gpui-symbols Components

[docs](https://gpui.dev) · [github](https://github.com/AprilNEA/gpui-symbols)

> `use: symbols: gpui-symbols` (guise feature-gated adapter).

<details><summary><strong>1 supported elements</strong></summary>

`Icon`

</details>

<details><summary><strong>3 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `SfSymbol` | rasterizer — `render()` yields RenderImage; `Icon` wraps it |
| `SymbolScale` | enum — small/medium/large for SfSymbol |
| `SymbolWeight` | enum — stroke weight for SfSymbol |

</details>

## ratatui Components

[docs](https://ratatui.rs/) · [github](https://github.com/ratatui/ratatui)

> ratatui-flavor base — `[x]` widgets render via `canvas:` mappings; the catalog also provides `VStack`/`HStack`/`Input` analogues ratatui lacks.

<details><summary><strong>14 supported elements</strong></summary>

`BarChart` `Block` `Calendar` `Canvas` `Chart` `Clear` (ratatui arm — clears its rect) `Gauge` `List` `Logo` `Mascot` `Paragraph` `Sparkline` `Table` `Tabs`

</details>

<details><summary><strong>13 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `Bar` | bar value type consumed by BarChart |
| `BarGroup` | grouping descriptor for BarChart |
| `Circle` | canvas shape — needs a Canvas draw context |
| `Fill` | layout constraint (`Constraint::Fill`), not a widget |
| `Line` | canvas shape — needs a Canvas draw context |
| `ListItem` | row value type for `List` |
| `Map` | canvas map layer — needs a Canvas draw context |
| `Points` | canvas shape — needs a Canvas draw context |
| `Rectangle` | canvas shape — needs a Canvas draw context |
| `Row` | row descriptor for `Table` |
| `Cell` | cell descriptor inside `Row` |
| `Scrollbar` | internal part of `ScrollArea` |
| `World` | map dataset for the canvas `Map` layer |

</details>

## iced Components

[docs](https://iced.rs/) · [github](https://github.com/iced-rs/iced)

> iced-flavor base — `[x]` widgets render via `canvas:` mappings (`Input`↔`TextInput`, `Switch`↔`Toggler`, `Progress`↔`ProgressBar`, `VStack`/`HStack`↔`Column`/`Row`).

<details><summary><strong>28 supported elements</strong></summary>

`Button` `Canvas` `Checkbox` `Column` `ComboBox` `Container` `Image` `KeyedColumn` `Markdown` `MouseArea` (iced arm — child wrapper + click hooks) `PaneGrid` `PickList` `ProgressBar` `Radio` `Row` `Rule` `Scrollable` `Slider` `Space` `Stack` `Svg` `Table` `Text` `TextEditor` `TextInput` `Toggler` `Tooltip` (iced arm — child + `tip`/`position` props) `VerticalSlider`

</details>

<details><summary><strong>7 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `Float` | widget wrapper needing an anchor+overlay callback |
| `Lazy` | widget wrapper keyed on a caller-supplied closure |
| `Pin` | widget wrapper anchoring children — Elm overlay semantics |
| `QRCode` | behind iced's `qr_code` cargo feature, not enabled on the runtime dep |
| `Responsive` | generic viewport switch — takes a layout closure, not props |
| `Sensor` | widget wrapper emitting window/resize sensor events |
| `Shader` | widget wrapper taking a wgpu shader Program |

</details>

## bezel Components

[docs](https://crates.io/crates/bezel) · [github](https://crates.io/crates/bezel)

> The `bezel` base catalog renders via `canvas:` mappings; the `bezel-*` crates are scaffolds with no Forge adapter yet.

<details><summary><strong>6 supported elements</strong></summary>

`VStack` (→ VStack) `HStack` (→ HStack) `Text` (→ Text) `Button` (→ Button) `Input` (→ Input) `Card` (→ GroupBox)

</details>


## adabraka Components

[docs](https://crates.io/crates/adabraka-ui) · [github](https://crates.io/crates/adabraka-ui)

> adabraka-ui (shadcn flavor) on the adabraka-gpui fork — the catalog's `[x]` elements render via `canvas:`/shared arms; the remaining upstream exports have no Forge adapter yet.

<details><summary><strong>84 supported elements</strong></summary>

`VStack` (→ VStack) `HStack` (→ HStack) `Card` (→ GroupBox) `Text` (→ Text) `Badge` (→ shared arm) `Avatar` (→ Avatar) `Button` (→ Button) `Input` (→ Input) `Checkbox` (→ Checkbox) `Switch` (→ Switch) `Select` (→ Select) `Progress` (→ Progress) `Alert` (→ shared arm) `Dialog` (→ Dialog) `Accordion` `AlertDialog` `AnimatedCounter` (cataloged as `AnimatedCounter` (canvas: mapping)) `AnimatedList` (cataloged as `AnimatedList` (canvas: mapping)) `AnimatedPresence` (cataloged as `AnimatedPresence` (canvas: mapping)) `AnimatedText` (cataloged as `AnimatedText` (canvas: mapping)) `AreaChart` `AvatarGroup` `BarChart` `BottomSheet` `Breadcrumbs` `Calendar` `ColorPicker` `Combobox` `Confetti` (cataloged as `Confetti` (canvas: mapping)) `ContextMenu` `CopyButton` `CropArea` (cataloged as `CropArea` (canvas: mapping)) `DataTable` `DatePicker` `Dock` (cataloged as `Dock` (canvas: mapping)) `DonutChart` `DrawerNavigation` (cataloged as `DrawerNavigation` (canvas: mapping)) `DropZone` (cataloged as `DropZone` (canvas: mapping)) `Dropdown` `Editor` `ExpandableCard` `FileTree` `FloatingActionButton` `Form` `Gauge` `GlassMorphism` (cataloged as `GlassMorphism` (canvas: mapping)) `HotkeyInput` (cataloged as `HotkeyInput` (canvas: mapping)) `HoverCard` `InfiniteScroll` `InlineEdit` `KBD` `LineChart` (cataloged as `LineChartAda` (canvas: mapping)) `MagneticButton` (cataloged as `MagneticButton` (canvas: mapping)) `Marquee` (cataloged as `Marquee` (canvas: mapping)) `Meteors` (cataloged as `Meteors` (canvas: mapping)) `NavigationMenu` (cataloged as `NavigationMenuWidget` (canvas: mapping)) `NumberInput` (cataloged as `NumberInputField` (canvas: mapping)) `PopoverMenu` (cataloged as `PopoverMenu` (canvas: mapping)) `RadarChart` (cataloged as `RadarChartAda` (canvas: mapping)) `Radio` (cataloged as `Radio` (canvas: mapping)) `RadioGroup` `RangeSlider` (cataloged as `RangeSlider` (canvas: mapping)) `Rating` `ResizablePanel` (cataloged as `ResizablePanel` (canvas: mapping)) `ResizablePanelGroup` (cataloged as `ResizablePanelGroup` (canvas: mapping)) `SearchInput` (cataloged as `SearchInput` (canvas: mapping)) `SegmentedNav` (cataloged as `SegmentedNav` (canvas: mapping)) `Separator` (cataloged as `Separator` (canvas: mapping)) `Sheet` `Skeleton` `Slider` `SortableList` (cataloged as `SortableList` (canvas: mapping)) `Spinner` (cataloged as `Spinner` (canvas: mapping)) `Spotlight` (cataloged as `Spotlight` (canvas: mapping)) `StatusBar` (cataloged as `StatusBarWidget` (canvas: mapping)) `Table` `Tabs` `TagInput` (cataloged as `TagInput` (canvas: mapping)) `TextReveal` (cataloged as `TextReveal` (canvas: mapping)) `TiltCard` (cataloged as `TiltCard` (canvas: mapping)) `Toggle` `TreeList` (cataloged as `TreeList` (canvas: mapping)) `TypeWriter` (cataloged as `TypeWriter` (canvas: mapping)) `ViewRouter` (cataloged as `ViewRouter` (canvas: mapping))

</details>

<details><summary><strong>39 reviewed — not standalone widgets</strong></summary>

| Export | Reason |
| --- | --- |
| `AccordionItem` | item descriptor for `Accordion` |
| `AnimatedSwitch` | implemented as `Switch` (component path maps to animated_switch::AnimatedSwitch) |
| `AreaChartSeries` | series value type for `AreaChart` |
| `AvatarItem` | row value type for `AvatarGroup` |
| `BarChartData` | data value type for `BarChart` |
| `BarChartSeries` | series value type for `BarChart` |
| `BreadcrumbItem` | item descriptor for `Breadcrumbs` |
| `ColumnDef` | column descriptor for `DataTable` |
| `Command` | palette item descriptor for `CommandPalette` |
| `ContextMenuItem` | menu item descriptor for `ContextMenu` |
| `DateFormat` | format enum for `DatePicker` |
| `DragData` | payload wrapper behind DropZone/Draggable |
| `Draggable` | drag-source primitive — `SortableList` covers reorderable lists |
| `DrawerSide` | enum — drawer anchor side |
| `DropdownItem` | item descriptor for `Dropdown` |
| `EditorLanguage` | prelude alias for editor `Language` enum |
| `FileNode` | node value type for `FileTree` |
| `LabelSide` | enum — toggle label side |
| `LayoutAnimation` | enum for layout-transition presets |
| `LineChartPoint` | point value type for LineChart |
| `LineChartSeries` | series value type for LineChart |
| `NavigationMenuItem` | item descriptor for NavigationMenu |
| `PopoverMenuItem` | item descriptor for PopoverMenu |
| `RadarDataset` | dataset value type for RadarChart |
| `RadioLayout` | enum — radio group layout |
| `RichBlock` | rich-text block AST for RichText |
| `RichInline` | rich-text inline AST for RichText |
| `SelectOption` | option descriptor for `Select`/`Combobox` |
| `SheetSide` | enum — sheet anchor edge |
| `SliderAxis` | enum — slider orientation |
| `StatusItem` | item descriptor for StatusBar |
| `TabItem` | item descriptor for `Tabs` |
| `TableColumn` | column descriptor for `Table` |
| `TableRow` | row descriptor for `Table` |
| `TextAnimation` | animation enum for `AnimatedText` |
| `ThemeTokens` | design-token struct consumed by the theme system |
| `ToastItem` | toast descriptor consumed by ToastManager |
| `ToastManager` | overlay host entity (Render, not RenderOnce) — mounted once at app root |
| `TreeNode` | node value type for TreeList/FileTree |

</details>

## forge built-ins

[docs](https://gpui-forge.rudeboy.dev) · [github](https://github.com/)

> `forge.*` — Forge tooling elements rendered by the runtime itself.

<details><summary><strong>7 supported elements</strong></summary>

`Preview` `Outline` `Inspector` `Files` `Palette` `Showcase` `Vars`

</details>


## template-lib Components

[docs](https://crates.io/crates/template-lib) · [github](https://crates.io/crates/template-lib)

> Template adapter scaffold (`forge-lib-template`).

<details><summary><strong>1 supported elements</strong></summary>

`Pill`

</details>

