# All Components

Every exported element/widget in each upstream library. `[x]` = implemented for Forge
documents (a runtime render arm, a `ForgeLibrary` adapter, or a `canvas:` mapping),
`[ ]` = upstream component with no Forge support yet. `→ X` marks the canvas analogue.
Upstream support/variant/helper types (`*State`, `*Event`, `*Variant`, `*Size`…)
are omitted for readability.


## GPUI-KIT Components

docs: https://gpui-kit.com/  
github: https://github.com/longbridge/gpui-kit

- [x] Accordion (https://gpui.rs/components/accordion)
- [x] AlertDialog (https://gpui.rs/components/alertdialog)
- [x] Alert (https://gpui.rs/components/alert)
- [x] Attachment (https://gpui.rs/components/attachment)
- [x] Avatar (https://gpui.rs/components/avatar)
- [x] Badge (https://gpui.rs/components/badge)
- [x] Bubble (https://gpui.rs/components/bubble)
- [x] Button (https://gpui.rs/components/button)
- [x] Calendar (https://gpui.rs/components/calendar)
- [x] Carousel (https://gpui.rs/components/carousel)
- [x] Chart (https://gpui.rs/components/chart) — LineChart, BarChart, PieChart
- [x] Checkbox (https://gpui.rs/components/checkbox)
- [x] Clipboard (https://gpui.rs/components/clipboard)
- [x] Collapsible (https://gpui.rs/components/collapsible)
- [x] ColorPicker (https://gpui.rs/components/colorpicker)
- [x] Combobox (https://gpui.rs/components/combobox)
- [x] Command (https://gpui.rs/components/command)
- [x] DataTable (https://gpui.rs/components/datatable) — alias of Table
- [x] DatePicker (https://gpui.rs/components/datepicker)
- [x] DescriptionList (https://gpui.rs/components/descriptionlist)
- [x] Dialog (https://gpui.rs/components/dialog)
- [x] Diff (https://gpui.rs/components/diff)
- [ ] Dock (https://gpui.rs/components/dock) — needs a panel registry
- [x] DropdownButton (https://gpui.rs/components/dropdownbutton)
- [x] Editor (https://gpui.rs/components/editor)
- [x] Empty (https://gpui.rs/components/empty)
- [x] Focus Trap (https://gpui.rs/components/focustrap)
- [x] Form (https://gpui.rs/components/form)
- [x] GridLayout (https://gpui.rs/components/gridlayout)
- [x] GroupBox (https://gpui.rs/components/groupbox)
- [x] HoverCard (https://gpui.rs/components/hovercard)
- [x] Icon (https://gpui.rs/components/icon)
- [x] Image (https://gpui.rs/components/image)
- [x] Input Group (https://gpui.rs/components/inputgroup)
- [x] Input (https://gpui.rs/components/input)
- [x] Kbd (https://gpui.rs/components/kbd)
- [x] Label (https://gpui.rs/components/label)
- [x] List (https://gpui.rs/components/list)
- [x] Marker (https://gpui.rs/components/marker)
- [ ] Menu (https://gpui.rs/components/menu) — native app menus are host-level. In-doc menus: ContextMenu/DropdownMenu
- [ ] MessageScroller (https://gpui.rs/components/messagescroller) — needs a per-row render callback
- [x] Message (https://gpui.rs/components/message)
- [x] Notification (https://gpui.rs/components/notification)
- [x] NumberInput (https://gpui.rs/components/numberinput)
- [x] OtpInput (https://gpui.rs/components/otpinput)
- [x] Pagination (https://gpui.rs/components/pagination)
- [ ] Plot (https://gpui.rs/components/plot) — chart internals — LineChart/BarChart/PieChart cover rendering
- [x] Popover (https://gpui.rs/components/popover)
- [x] Progress (https://gpui.rs/components/progress)
- [x] Questionnaire (https://gpui.rs/components/questionnaire)
- [x] Radio (https://gpui.rs/components/radio)
- [x] Rating (https://gpui.rs/components/rating)
- [x] Resizable (https://gpui.rs/components/resizable)
- [ ] Root View (https://gpui.rs/components/rootview) — window-level — handled by the app shell
- [x] Scrollable (https://gpui.rs/components/scrollable)
- [x] Select (https://gpui.rs/components/select)
- [x] Settings (https://gpui.rs/components/settings)
- [x] Sheet (https://gpui.rs/components/sheet) — via `Dialog#id` + `sheet: true`
- [x] Shimmer (https://gpui.rs/components/shimmer)
- [x] Sidebar (https://gpui.rs/components/sidebar)
- [x] Skeleton (https://gpui.rs/components/skeleton)
- [x] Slider (https://gpui.rs/components/slider)
- [ ] Speech (https://gpui.rs/components/speech) — hardware speech recognizer
- [x] Spinner (https://gpui.rs/components/spinner)
- [x] StatusBar (https://gpui.rs/components/statusbar)
- [x] Stepper (https://gpui.rs/components/stepper)
- [x] Switch (https://gpui.rs/components/switch)
- [x] Table (https://gpui.rs/components/table)
- [x] Tabs (https://gpui.rs/components/tabs)
- [x] Tag (https://gpui.rs/components/tag)
- [x] TextView (https://gpui.rs/components/textview)
- [x] Textarea (https://gpui.rs/components/textarea)
- [ ] Theme (https://gpui.rs/components/theme) — window-level theme — handled by the app shell
- [x] TimeField (https://gpui.rs/components/timefield)
- [x] TitleBar (https://gpui.rs/components/titlebar)
- [x] Toggle (https://gpui.rs/components/toggle) — alias of Switch
- [x] Toolbar (https://gpui.rs/components/toolbar)
- [ ] Tooltip (https://gpui.rs/components/tooltip) — attribute-level (`.tooltip()` on any element) — no standalone element
- [x] Tree (https://gpui.rs/components/tree)
- [x] VirtualList (https://gpui.rs/components/virtuallist) — alias of List

> Remaining exports of `gpui_kit::component` (the upstream `gpui-component` crate re-exported through gpui-kit — same surface, not a separate set) reviewed and deliberately not cataloged:
- [ ] Cancel — gpui `Action` — dialog cancel, not a widget
- [ ] Confirm — gpui `Action` — dialog confirm, not a widget
- [ ] Date — value type, not a widget — `DatePicker`/`Calendar` produce it
- [ ] DiffConflict — merge-conflict value type for the diff viewer
- [ ] DiffConflictResolution — enum — which side resolves a conflict
- [ ] DiffFile — diff document value type behind DiffView
- [ ] DiffFileStatus — enum — added/modified/deleted for DiffFile
- [ ] DiffSide — enum — old/new side of a diff hunk
- [ ] Disableable — trait — `.disabled(bool)` support
- [ ] History — undo/redo entity behind inputs
- [ ] HourCycle — enum — 12h/24h for TimeField/DatePicker
- [ ] Matcher — fuzzy-match trait behind SearchableList
- [ ] NumberStep — enum — NumberInput step behavior
- [ ] PlotHover — chart hover value type behind interactive charts
- [ ] PopupMenu — menu entity — `PopupMenu::build(cx)` attaches to a trigger, not a standalone element
- [ ] PopupMenuItem — menu item descriptor consumed by PopupMenu
- [x] RadarChart — implemented (base arm, `data` series)
- [ ] RadarLabel — axis label value type for RadarChart
- [ ] RoleOverride — a11y role-override trait
- [ ] Rope — rope text buffer behind Editor/inputs
- [ ] RopeLines — rope line iterator behind Editor
- [x] SankeyChart — implemented (`nodes`/`links` props)
- [ ] SankeyLabel — edge label descriptor — SankeyChart builds it
- [ ] SearchableGroup — grouped-list section type for SearchableList
- [ ] SearchableListDelegate — trait — supplies SearchableList rows
- [ ] SearchableListItem — list row value type for SearchableList
- [ ] SearchableVec — searchable row container for SearchableList
- [ ] Selectable — trait — `.selected(bool)` support
- [ ] Sizable — trait — `.size(...)` support
- [ ] SliderScale — enum — linear/log slider scale
- [ ] SpeechStatus — enum — speech-button state (recording/processing)
- [ ] StyleSized — trait — style-driven sizing
- [ ] TimePrecision — enum — TimeField granularity
- [ ] TimeSegment — enum — hour/minute/second segment of TimeField
- [ ] UndoHistory — undo stack entity behind inputs
- [x] WindowBorder — implemented (child wrapper)

## GPUI-KIT sub-elements (catalog)

docs: https://gpui-kit.com/  
github: https://github.com/longbridge/gpui-kit

> Child elements — render inside their parent arms.

- [x] Radio
- [x] Tab
- [x] MenuItem
- [x] MenuSeparator
- [x] BreadcrumbItem
- [x] DescriptionItem
- [x] StepperItem
- [x] AccordionItem
- [x] MessageAvatar
- [x] MessageHeader
- [x] MessageContent
- [x] MessageFooter
- [x] TableHeader
- [x] TableBody
- [x] TableFooter
- [x] TableCaption
- [x] TableRow
- [x] ResizablePanel
- [x] SidebarHeader
- [x] SidebarFooter
- [x] SidebarMenu
- [x] SidebarMenuItem
- [x] TreeItem
- [x] CommandItem
- [x] CommandGroup
- [x] CommandSeparator
- [x] QuestionnaireItem
- [x] QuestionnaireChoice
- [x] Sheet
- [x] Box
- [x] VStack
- [x] HStack
- [x] SettingPage
- [x] SettingsGroup
- [x] SettingItem
- [x] Text
- [x] RadioGroup
- [x] Separator
- [x] DropdownMenu
- [x] Link
- [x] Breadcrumb
- [x] ProgressCircle
- [x] AvatarGroup
- [x] FormField
- [x] TableHead
- [x] TableCell
- [x] Grid
- [x] FocusTrap
- [x] ContextMenu
- [x] InputGroup
- [x] InputGroupInput
- [x] InputGroupTextarea
- [x] InputGroupAddon
- [x] InputGroupText
- [x] InputGroupButton
- [x] LineChart
- [x] BarChart
- [x] PieChart
- [x] SwitchButton

## ely Components

docs: https://elygpui.com/  
github: https://github.com/ELY-Open-Source/Ely-GPUI-Components

> The ely site advertises 1279 items (widgets + ~900 icons). Below is every exported widget type in `ely-core` 0.2 plus the `ely-terminal` 0.1.1 crate — the `[x]` entries render natively via `forge-lib-ely` (`use: ely: ely-core`). The rest have no Forge adapter support yet. `ely-terminal.Terminal` is native-only (pty). On wasm the `Terminal` element is absent by `cfg`.

- [x] Accordion
- [x] AccordionItem
- [x] ActionBar
- [x] Activity
- [x] ActivityBar
- [x] ActivityFeed
- [x] Alert
- [x] AlertDialog
- [x] Anchor
- [x] AnimatePresence
- [x] AnimatedGradient
- [x] AnsiText — `ely-terminal` — ANSI-colored text, SGR only
- [x] AppShell
- [x] AspectRatio
- [ ] AsyncView — needs a `Future` + `Context` — async entity, not declarative
- [x] Avatar
- [x] AvatarGroup
- [x] Backdrop
- [x] Badge
- [x] Banner
- [x] Barcode
- [x] BeforeAfter
- [x] Blink
- [x] Blockquote
- [x] Breadcrumb
- [x] BulkActionBar
- [x] Button
- [x] ButtonGroup
- [ ] COUNTRIES — `(&str,&str,&str,&str)` constant table — PhoneInput country list
- [x] Calendar
- [x] Callout
- [x] Caption
- [x] Card
- [x] CardHeader
- [x] Carousel
- [ ] Cascade — Cascader option node — built from `options` props
- [x] Cascader
- [ ] Change — change-record value type behind inputs
- [ ] Changeable — trait — widgets emit Change events
- [x] Changelog
- [x] Checkbox
- [x] CheckboxCard
- [x] CheckboxGroup
- [ ] Close — `Rc<dyn Fn>` dismiss-callback alias
- [x] Code
- [x] Collapsible
- [x] ColorSwatch
- [ ] Column — column descriptor — tables build it from `columns` props
- [x] ColumnShell
- [ ] Command — palette item descriptor — `value`/`label`/`keys` consumed by CommandPalette `groups`
- [ ] CommandBlock — `ely-terminal` — command + output block (Running/Ok/Error states)
- [ ] CommandHistory — `ely-terminal` — `PastCommand` list
- [x] Comparison
- [x] Confetti
- [x] ConfirmButton
- [x] ConfirmDialog
- [x] ConnectionStatus
- [ ] Connectivity — ConnectionStatus state enum — set via `state` prop
- [x] Container
- [x] ContextMenu
- [ ] Copy — gpui `Action` — copy command, not a widget
- [x] CountBadge
- [x] Countdown
- [ ] Crumb — Breadcrumb item descriptor — built from `items` props
- [ ] Cut — gpui `Action` — cut command, not a widget
- [x] DataGrid
- [x] DataTable
- [x] DatePicker
- [x] DateRangePicker
- [x] DateTimePicker
- [x] DescriptionList
- [x] Dialog
- [x] DirectoryListing
- [ ] Disableable — trait — `disabled` prop support
- [x] Dock
- [ ] DockLayout — layout model Dock owns internally
- [x] DockPanel
- [x] DockSide
- [x] DotBadge
- [x] Drawer
- [ ] Drawing — internal pen-stroke entity behind SignaturePad
- [ ] DropAt — drop-position enum for drag targets
- [x] DropZone
- [ ] Dropdown — `#[doc(hidden)]` picker popup state, owned by date pickers
- [x] DropdownMenu
- [x] DurationPicker
- [ ] Editing — editor-state enum behind InlineEdit
- [ ] EditorTab — EditorTabs tab descriptor — built from `items` props
- [x] EditorTabs
- [x] Ellipsis
- [x] EllipsisTooltip
- [x] EmailInput
- [x] EmojiPicker
- [ ] Entrance — Transition/Stagger `entrance` enum (Fade/Rise/Slide)
- [x] ErrorBoundary
- [x] ErrorView
- [x] ExpressionInput
- [x] ExternalLink
- [x] FieldArray
- [x] Fieldset
- [x] FileInput
- [x] FileTree
- [ ] Finder — internal glyph-finder machinery of EmojiPicker/IconPicker
- [x] Flash
- [x] Flip
- [x] FloatingToolbar
- [ ] Folder — `#[doc(hidden)]` tree builder backing FileTree
- [x] FontPicker
- [x] Frame
- [x] Gallery
- [x] Gauge
- [x] Glow
- [x] GradientEditor
- [x] GradientText
- [ ] Grid — internal glyph grid of EmojiPicker/IconPicker
- [ ] HUE_NAMES — hue-name constant table for color pickers
- [x] Heading
- [x] HeatmapTable
- [x] HelpTooltip
- [x] Highlight
- [ ] History — undo/redo history entity behind text inputs
- [x] IconButton
- [x] IconPicker
- [x] Image
- [x] InfiniteList
- [x] InlineEdit
- [x] InlineMessage
- [x] Input
- [x] InputAddon
- [x] InputGroup
- [x] IntersectionObserver
- [x] Kbd
- [x] KbdCombo
- [x] KeyValueInput
- [x] KpiCard
- [ ] LOCALES — locale constant table
- [x] Label
- [x] LazyLoad
- [x] Lightbox
- [x] Link
- [x] List
- [x] ListInput
- [x] ListItem
- [x] LoadMore
- [x] LoadingOverlay
- [x] LogViewer — `ely-terminal` — 'at|LEVEL|target|message' `lines`
- [x] Marquee
- [x] MaskedInput
- [x] Masonry
- [x] MasterDetail
- [x] Measure
- [ ] Menu — menu model — DropdownMenu/ContextMenu own it via OwnedMenu
- [x] MenuItem — base.Menu
- [x] Meter
- [x] MiddleEllipsis
- [ ] Mix — gradient color-stop pair for GradientEditor
- [x] MonthPicker
- [x] NavGroup
- [x] NavItem
- [x] Notification
- [x] NotificationCenter
- [x] NumberInput
- [x] OfflineIndicator
- [ ] OnClimb — `Rc<dyn Fn>` breadcrumb-climb callback alias
- [ ] OnFold — `Rc<dyn Fn>` section-fold callback alias
- [x] OverflowMenu
- [x] Overline
- [ ] PICTURES — sample-picture constant table for demos
- [x] Page
- [x] Pagination
- [x] PaneGroup
- [x] PaneId
- [x] PaneLayout
- [x] Panel
- [x] Paragraph
- [x] ParticleBackground
- [x] PasswordInput
- [ ] Paste — gpui `Action` — paste command, not a widget
- [x] PathInput
- [x] Peek
- [x] PhoneInput
- [ ] Pick — selection enum + `Rc` pick-callback alias
- [ ] PieItem — PieMenu sector descriptor
- [x] PieMenu
- [ ] Place — positioning enum (Center/Left/Right/Top/Bottom)
- [ ] Presence — Avatar presence enum — set via `presence` prop
- [ ] ProcessList — `ely-terminal` — `Process` rows. Needs live process data
- [x] ProgressBar
- [x] ProgressRing
- [x] PromptDialog
- [x] PropertyGrid
- [x] PropertyGroup
- [x] Pulse
- [x] QrCode
- [x] QuarterPicker
- [x] QuickOpen
- [x] QuickSwitcher
- [x] Radio
- [x] RadioCard
- [x] RadioGroup
- [ ] Raised — elevation/style enum
- [x] RangeSlider
- [ ] Refresh — RefreshIndicator `state` enum (Idle/Pulling/Refreshing/Done)
- [x] RefreshIndicator
- [x] RegexInput
- [x] RelativeDatePicker
- [x] Release
- [x] Ripple
- [x] SavingIndicator
- [ ] ScrollModel — virtual-scroll state model behind VirtualList/InfiniteList
- [ ] ScrollPlace — scroll-position value type
- [x] ScrollShadow
- [x] ScrollToTop
- [x] ScrubInput
- [x] SearchableMenu
- [x] Section
- [x] Sections
- [ ] SelectAll — gpui `Action` — select-all command, not a widget
- [ ] Selectable — trait — selection support
- [x] SelectableList
- [x] Shake
- [x] Sheet
- [x] ShellColumn
- [x] ShellSelector — `ely-terminal` — 'name=program' `shells`
- [x] Shimmer
- [x] ShimmerText
- [x] SignaturePad
- [x] SimpleGrid
- [ ] Sizable — trait — `size` prop support
- [x] Skeleton
- [x] SkeletonAvatar
- [x] SkeletonCard
- [x] SkeletonTable
- [x] SkeletonText
- [x] Slider
- [x] Sparkline
- [x] Spinner
- [x] SplitButton
- [x] SplitPane
- [ ] Spot — Spotlight/Tour target descriptor needing live Bounds
- [x] Spreadsheet
- [x] Stagger
- [x] Stars
- [x] Statistic
- [x] StatusBar
- [x] StatusBarItem
- [x] StatusMessage
- [x] Steps
- [x] StickyHeader
- [ ] Stroke — `Vec<Point<Pixels>>` alias — signature ink data
- [x] Subtitle
- [x] Suspense
- [x] SwipeableListItem
- [x] SyncStatus
- [ ] Syntax — static language-rule table behind CodeInput (`syntax_of`)
- [x] TabBar
- [ ] TabPlacement — Tabs placement enum (Top/Bottom/Left/Right)
- [x] Table
- [x] TableOfContents
- [x] Tabs
- [x] Tag
- [x] Terminal — `ely-terminal` — pty `command`/`cwd` or `output` replay. Native only
- [x] TerminalToolbar — `ely-terminal` — clear/stop/split/`shells` picker
- [ ] TextInput — input state entity. Forge inputs manage it internally
- [x] TimePicker
- [x] Timeline
- [x] TimelineItem
- [x] Title
- [x] TitleBar
- [x] Toast
- [x] ToastViewport
- [x] Toaster
- [x] ToggleButton
- [x] ToggleGroup
- [x] ToggleItem
- [ ] Tone — severity/color enum — set via `tone` props
- [x] Toolbar
- [x] ToolbarGroup
- [x] ToolbarSeparator
- [ ] Tooltip — content factory (`Tooltip::text`) for TooltipTrigger, not a widget
- [x] Transform
- [x] Tree
- [ ] TreeNode — tree node value type — built from `nodes` props
- [x] TreeSelect
- [x] TreeTable
- [x] TrendIndicator
- [x] Typewriter
- [x] TypingIndicator
- [x] UpdateBanner
- [x] UpdateDialog
- [ ] Upload — row value type consumed by UploadList `uploads`
- [x] UploadList
- [x] UrlInput
- [x] UsageBar
- [x] UserChip
- [x] Viewport
- [x] VirtualList
- [ ] WORKSPACE_VERSION — workspace-schema version constant
- [x] Watermark
- [x] Well
- [x] WindowControls
- [x] Wizard
- [ ] Workspace — dock capture/apply session JSON — `to_json`/`apply`, not a widget
- [x] YearPicker
- [x] ZStack
- [x] ZoomControl
- [ ] ZoomStep — zoom-level value type behind ZoomControls
- [x] Divider
- [x] CopyButton
- [x] SegmentedControl
- [x] EmptyState
- [x] Switch
- [x] Rating
- [x] Stepper
- [x] Icon


- [x] AvatarUpload
- [x] CodeInput
- [x] ColorPicker
- [x] Combobox
- [x] CronEditor
- [x] DirtyIndicator
- [x] EyeDropper
- [x] FormError
- [x] FormField
- [x] FormLabel
- [x] FormSection
- [x] InlineForm
- [x] JsonInput
- [x] ListBox
- [x] MentionInput
- [x] SearchInput
- [x] TagInput
- [x] TimezoneSelect
- [x] TransferList
- [x] UnitInput
- [x] WeekPicker

- [x] Outline
- [x] SortableList

- [x] GroupedList

- [x] BackForwardNavigation
- [x] GoToLine
- [x] NavigationMenu
- [x] TabOverflowMenu

- [x] CrashReporter
- [x] HoverCard
- [x] Popover
- [x] ResizeBorder
- [x] Vibrancy
- [x] WindowSwitcher

- [ ] MenuBar — menu items need a boxed gpui `Action`, not declarative handlers
- [ ] Tour — TourStep needs live `Bounds<Pixels>` of a target element
- [ ] Spotlight — same: step target is a `Bounds<Pixels>`, no doc-level geometry
- [ ] TourStep — value type holding a `Bounds<Pixels>` target
- [ ] WindowTab — descriptor consumed by TabBar, not standalone
- [ ] ActivityItem — descriptor consumed by ActivityBar children
- [ ] WindowManager — app-level window host registry, not a widget
- [ ] Hosted — wrapper type for `open`-spawned window content
- [ ] MiniWindow — spawns a real hosted window, not in-tree render
- [ ] Corner — window-corner value type
- [ ] AboutDialog — `.open()` spawns a hosted window, not an element
- [ ] SplashScreen — `.open()` spawns a hosted window, not an element
- [ ] Sections (navigation) — scroll-state entity backing TOC/Anchor, not a widget
- [ ] DragRegion/drag_region — helper fn, not a component
- [ ] media_button — helper fn, not a component
- [ ] menu_under — helper fn, not a component
- [ ] guard_close — window-lifecycle helper fn, not a component

- [x] AnimatedNumber
- [x] CurrencyText
- [x] DateTimeText
- [x] DurationText
- [x] Emoji
- [x] FileSizeText
- [x] Link
- [x] PivotTable
- [x] PluralText
- [x] RelativeTime
- [x] SelectableText
- [x] TableToolbar

- [ ] FilterBuilder — interactive rule editor needing owner callbacks
- [ ] SortBuilder — interactive sort editor needing owner callbacks
- [ ] FilterRule — rule value type consumed by DataTable.filters
- [ ] SortKey — sort value type consumed by DataTable.sorts
- [ ] Test — filter-operator enum, not a widget
- [ ] to_csv — helper fn, not a component
- [ ] Aggregate — cell-aggregation enum, not a widget
- [ ] Align — column-align enum, not a widget
- [ ] Cell — table cell value type (encoded in `rows` props)
- [ ] Row — table row value type (built from `rows` props)
- [ ] TreeRow — tree row value type (built from `rows` props)
- [ ] text_width — helper fn, not a component
- [ ] literal — helper fn, not a component
- [ ] tabular — helper fn, not a component
- [ ] fresh — helper fn, not a component
- [ ] relative_text — helper fn, not a component
- [ ] LEADING — typography constant

- [x] ProgressBar
- [x] Reorder
- [x] Transition

- [x] `TooltipTrigger`
- [x] `CommandPalette`
## guise Components

docs: https://docs.rs/guise  
github: https://github.com/wess/guise

> Vanilla-gpui flavor base — `base: guise` on crates.io `gpui 0.2.2`. Below is every exported widget type in `guise-ui` 1.10 — `[x]` entries are mapped by `catalogs/guise.catalog.yaml` (`Stack`/`Group` surface as Forge `VStack`/`HStack`). Each unchecked entry carries a one-line reason it stays unmapped.

- [x] About
- [x] Accordion
- [x] ActionIcon
- [x] AIChatView
- [x] AICitation
- [x] AIComposer
- [x] AICost
- [x] AIMessage
- [x] AIModelPicker
- [x] AIReasoning
- [x] AISettings
- [x] AISources
- [x] AIStreamingText
- [x] AIThinking
- [x] AITokenMeter
- [x] AIToolCall
- [ ] AIModel — model descriptor consumed by `AIModelPicker`, not a widget
- [ ] AIPricing — pricing descriptor consumed by `AICost`, not a widget
- [ ] AISource — source descriptor consumed by `AISources`/`AIChatView`, not a widget
- [ ] AITurn — turn descriptor consumed by `AIChatView`, not a widget
- [ ] AITurnTool — tool descriptor consumed by `AITurn`, not a widget
- [ ] AIUsage — token-usage descriptor consumed by `AICost`, not a widget
- [x] Alert
- [x] Anchor
- [ ] Animated — animation primitive — wraps an element tree, not a standalone widget
- [ ] Animator — drives `Animated` — construct it from Forge state instead
- [x] AppShell
- [x] Autocomplete
- [x] Avatar
- [x] AvatarGroup
- [x] Badge
- [x] BarChart
- [x] Blockquote
- [x] Breadcrumbs
- [x] Button
- [x] Calendar
- [x] Card
- [x] Carousel
- [x] Center
- [x] Checkbox
- [x] CheckboxGroup
- [x] Chip
- [x] CloseButton
- [x] Code
- [ ] Collapse — animation primitive — use `Spoiler` for expandable content
- [x] ColorInput
- [x] Combobox
- [x] ConfirmModal
- [x] Container
- [x] ContextMenu
- [x] CopyButton
- [x] DataView
- [ ] Date — value type, not a widget — `DatePicker`/`Calendar` produce it
- [x] DatePicker
- [ ] DevTools — in-app debug panel — developer tooling, not a document element
- [x] Divider
- [ ] Draggable — drag-source primitive — `SortableList` covers reorderable lists
- [x] Drawer
- [ ] DropTarget — drop-target primitive — needs host drag state. `Dropzone` covers file drops
- [x] Dropzone
- [x] Editor
- [x] Field
- [x] FileInput
- [ ] GpuView — raw wgpu surface — needs a caller-supplied `GpuScene` render callback
- [x] Group
- [x] HoverCard
- [x] Icon
- [x] Image
- [x] Indicator
- [x] Kbd
- [x] LineChart
- [x] List
- [x] Loader
- [x] LoadingOverlay
- [x] Markdown
- [x] MarkdownEditor
- [x] Mark
- [x] Menu
- [x] MenuBar
- [ ] MenuColumn — internal layout primitive of `Menu`/`MenuBar`
- [x] Modal
- [ ] Motioned — animation wrapper — applies motion curves to a child element
- [x] NavigationMenu
- [x] NavLink
- [x] Notification
- [x] NumberInput
- [ ] OverlayHost — internal mounting point — overlays host themselves
- [x] Pagination
- [ ] PaneGroup — unconstructable — its `ItemId` is `pub(crate)` upstream
- [x] Panel
- [x] Paper
- [x] PasswordInput
- [x] PieChart
- [x] PinInput
- [x] Popover
- [ ] Presence — exit-animation primitive — wraps elements, not a standalone widget
- [x] Progress
- [x] Radio
- [x] RadioGroup
- [x] RangeSlider
- [x] Rating
- [ ] ResizeHandles — internal part of `SplitPanel`
- [ ] Responsive — generic viewport switch — takes a layout closure, not props
- [x] RingProgress
- [x] ScatterChart
- [x] ScrollArea
- [ ] Scrollbar — internal part of `ScrollArea`
- [x] SegmentedControl
- [x] Select
- [ ] Sequence — animation sequencing primitive
- [ ] SettingsPage — page descriptor consumed by `SettingsView`, not a widget
- [x] SettingsRow
- [x] SettingsSection
- [x] SettingsView
- [x] SimpleGrid
- [x] Skeleton
- [x] Slider
- [x] SortableList
- [x] Space
- [x] Sparkline
- [x] SplitPanel
- [x] Spoiler
- [x] Spotlight
- [x] Stack
- [ ] Stagger — animation sequencing primitive
- [x] StatusBar
- [x] Stepper
- [x] Switch
- [x] TabBar
- [x] Table
- [x] TableView
- [x] Tabs
- [x] TagsInput
- [x] Text
- [x] TextArea
- [ ] TextEdit — editor value type (cursor/undo model), not a widget
- [x] TextInput
- [x] ThemeIcon
- [x] ThemePicker
- [ ] Time — value type, not a widget — `TimePicker` produces it
- [x] TimePicker
- [x] Timeline
- [x] Title
- [x] ToastStack
- [x] Tooltip
- [x] Tour
- [x] Transfer
- [ ] Transition — animation primitive
- [x] TreeView
- [ ] UpdateNotice — needs a live `Updater`/`UpdateOutcome` — host update infrastructure
- [ ] UpdatePrompt — needs a live `Updater`/`Release` — host update infrastructure
- [ ] VideoFeed — frame source consumed by `VideoView`, not a standalone widget
- [ ] VideoFrame — single frame consumed by `VideoView`, not a widget
- [x] VideoView
- [x] VirtualList
- [x] WebView
- [x] WindowControls

## gpui-flow Components

docs: https://github.com/pacifio/gpui-flow  
github: https://github.com/pacifio/gpui-flow

> `use: gflow: gpui-flow` — node-graph canvas, rendered by `forge-lib-flow`.

- [x] Canvas

## ferrum-flow Components

docs: https://crates.io/crates/ferrum-flow  
github: https://crates.io/crates/ferrum-flow

> `use: flow: ferrum-flow` (guise feature-gated adapter).

- [x] Canvas
- [ ] ActiveNodeDrag — in-flight drag state behind Flow's node dragging
- [ ] Command — undo/redo command trait — app supplies implementations
- [ ] CompositeCommand — batch of `Command`s — internal undo machinery
- [ ] ContextMenuCanvasExtra — context-menu payload for Flow's canvas
- [ ] CreateEdge — command/descriptor used to insert edges
- [ ] CreatePort — command/descriptor used to insert ports
- [ ] GraphChange — change record in the graph history log
- [ ] GraphOp — enum — graph mutation ops behind `Flow`
- [ ] HistoryProvider — trait — pluggable undo backends for Flow
- [ ] LocalHistory — default in-memory `HistoryProvider` impl
- [ ] NODE_DRAG_TICK_INTERVAL — const — drag-update cadence
- [ ] NodeRenderer — trait — app-supplied node body renderer for Flow
- [ ] ToastLevel — enum — toast severity for Flow's notices
- [ ] ToastMessage — notice value type for Flow's toast overlay

## base-gpui Components

docs: https://luketandjung.github.io/base-gpui/overview/quick-start.html  
github: https://github.com/LukeTandjung/base-gpui

> Headless Base UI port (vendored at `vendor/base-gpui`, pinned to a Zed gpui rev) — `use: base: base-gpui`, rendered by `forge-lib-base`. Components ship unstyled as compound parts (`XxxRoot` + named parts — per-part inventory under "base-gpui parts" below). The adapter composes them into the `[x]` Forge elements with default chrome via `style_with_state`. Support types (`*Props`, `*Runtime`, `*Context`, `*StyleState`, `*Child`, `*ChildWiring`, `*Metadata`, `*Details`, `*Outcome`, `*Handler`, action/key-context consts, option enums like `AutocompleteMode`/`MenubarOrientation`, and helper fns like `input`/`scrollbar`/`modal_backdrop`/`point_in_*`/`normalize_otp_value`) are omitted for readability, as are the `Autocomplete*`/`ContextMenu*` aliases re-exporting Combobox/Menu parts. Event wiring: `on change:`/`on open:`/`on submit:`/`on activate:`/`on input:`/`on complete:` fire Forge handlers and write `bind $path`/`$path`-bound props back to state — `base.Input`/`base.ToolbarInput`/`FieldControl` value callbacks take `Fn(SharedString)` with no `&mut App`, so text-input `change` is not wireable upstream.

- [x] Accordion — AccordionRoot + Item/Header/Trigger/Panel parts
- [x] AlertDialog — AlertDialogRoot + Trigger/Popup parts (Action/Cancel unmapped)
- [x] Autocomplete — AutocompleteRoot + Input/InputGroup/Clear + Combobox popup parts (Trigger/Icon/Value/ItemIndicator unmapped)
- [x] Avatar — AvatarRoot + Image/Fallback
- [x] Button — ButtonRoot
- [x] Checkbox — CheckboxRoot + Indicator
- [x] CheckboxGroup — CheckboxGroup + Checkbox children
- [x] Collapsible — CollapsibleRoot + Trigger/Panel parts
- [x] Combobox — ComboboxRoot + InputGroup/Input/Clear/Trigger/Icon/Portal/Positioner/Popup/List/Item/ItemIndicator parts (Value/Chips/Backdrop/Arrow/Collection/Empty/Group/GroupLabel/Separator/Status/Label unmapped)
- [x] ContextMenu — ContextMenuRoot + Trigger + Menu parts (re-exported as `ContextMenu*`)
- [x] Dialog — DialogRoot + Trigger/Portal/Backdrop/Viewport/Popup/Title/Description/Close parts
- [x] Drawer — DrawerRoot + Portal/Backdrop/Viewport/Popup/Content parts (Provider/SwipeArea/Indent/IndentBackground unmapped)
- [x] Field — FieldRoot + Label/Description/Error parts (Item/Control/Validity unmapped)
- [x] Fieldset — FieldsetRoot + Legend parts
- [x] Form — single-element compound (validation/submission context)
- [x] Input
- [x] Menu — MenuRoot + Trigger/Portal/Positioner/Popup/Item/LinkItem/CheckboxItem/CheckboxItemIndicator/RadioGroup/RadioItem/RadioItemIndicator/Group/GroupLabel/Separator/SubmenuRoot/SubmenuTrigger parts (Backdrop/Arrow unmapped)
- [x] Menubar — single-element compound (base.Menu children wired via MenubarChild)
- [x] Meter — MeterRoot + Label/Track/Indicator/Value
- [x] NavigationMenu — NavigationMenuRoot + List/Item/Trigger/Content/Portal/Positioner/Popup/Viewport parts (Link/Icon/Backdrop/Arrow unmapped)
- [x] NumberField — NumberFieldRoot + Group/Input/Increment/Decrement parts (ScrubArea unmapped)
- [x] OtpField — OTPFieldRoot + Input slots
- [x] Popover — PopoverRoot + Trigger/Portal/Positioner/Popup/Title/Description parts (Backdrop/Viewport/Arrow/Close unmapped)
- [x] PreviewCard — PreviewCardRoot + Trigger/Portal/Positioner/Viewport/Popup parts (Backdrop/Arrow unmapped)
- [x] Progress — ProgressRoot + Label/Track/Indicator/Value
- [x] RadioGroup — RadioGroupRoot + Radio/Indicator parts
- [x] ScrollArea — ScrollAreaRoot + Viewport/Content/Scrollbar/Thumb parts (Corner unmapped)
- [x] Select — SelectRoot + Trigger/Value/Icon/Portal/Positioner/Popup/List/Item/ItemText/ItemIndicator parts (Backdrop/Arrow/Group/GroupLabel/Label/Separator/ScrollArrows unmapped)
- [x] Separator
- [x] Slider — SliderRoot + Control/Track/Indicator/Thumb
- [x] Switch — SwitchRoot + Thumb
- [x] Tabs — TabsRoot + List/Tab/Panel parts (Indicator unmapped)
- [ ] Toast — ToastProvider + Portal/Viewport/Root/Content/Title/Description/Action/Close parts
- [x] Toggle
- [x] ToggleGroup — ToggleGroup + Toggle children
- [x] Toolbar — ToolbarRoot + Group/Button/Input/Link/Separator parts
- [x] Tooltip — TooltipRoot + Trigger/Portal/Positioner/Popup parts (Provider/Viewport/Arrow unmapped)

## base-gpui parts (compound)

docs: https://luketandjung.github.io/base-gpui/overview/quick-start.html  
github: https://github.com/LukeTandjung/base-gpui

> Compound child parts — each renders inside its component's arm (`*Root` is the anchor the adapter builds. `*Portal`/`Positioner`/`Backdrop`/`Arrow`/`Viewport`/`Provider` are overlay plumbing it owns). `[x]` = already composed by a `base.*` element.

- [x] AccordionHeader — base.Accordion
- [x] AccordionItem — base.Accordion
- [x] AccordionPanel — base.Accordion
- [x] AccordionRoot — base.Accordion
- [x] AccordionTrigger — base.Accordion
- [x] AlertDialogPopup — base.AlertDialog
- [x] AlertDialogRoot — base.AlertDialog
- [x] AlertDialogTrigger — base.AlertDialog
- [x] AutocompleteRoot — base.Autocomplete
- [ ] AutocompleteValue — Autocomplete part
- [x] AvatarFallback — base.Avatar
- [x] AvatarImage — base.Avatar
- [x] AvatarRoot — base.Avatar
- [x] ButtonRoot — base.Button
- [x] CheckboxGroup — base.CheckboxGroup
- [x] CheckboxIndicator — base.Checkbox
- [x] CheckboxRoot — base.Checkbox
- [x] CollapsiblePanel — base.Collapsible
- [x] CollapsibleRoot — base.Collapsible
- [x] CollapsibleTrigger — base.Collapsible
- [x] ComboboxArrow — base.Combobox / base.Autocomplete (`arrow` prop)
- [x] ComboboxBackdrop — base.Combobox / base.Autocomplete (`backdrop` prop)
- [ ] ComboboxChip — Combobox part
- [ ] ComboboxChipRemove — Combobox part
- [ ] ComboboxChips — Combobox part
- [x] ComboboxClear — base.Combobox
- [ ] ComboboxCollection — Combobox part
- [x] ComboboxEmpty — base.Combobox / base.Autocomplete (`empty` prop)
- [x] ComboboxGroup — base.OptionGroup
- [x] ComboboxGroupLabel — base.OptionGroup
- [x] ComboboxIcon — base.Combobox
- [x] ComboboxInput — base.Combobox
- [x] ComboboxInputGroup — base.Combobox
- [x] ComboboxItem — base.Combobox
- [x] ComboboxItemIndicator — base.Combobox
- [x] ComboboxLabel — base.Combobox (`label` prop)
- [x] ComboboxList — base.Combobox
- [x] ComboboxPopup — base.Combobox
- [x] ComboboxPortal — base.Combobox
- [x] ComboboxPositioner — base.Combobox
- [x] ComboboxRoot — base.Combobox
- [x] ComboboxSeparator — base.OptionSeparator
- [x] ComboboxStatus — base.Combobox / base.Autocomplete (`status` prop)
- [x] ComboboxTrigger — base.Combobox
- [ ] ComboboxValue — Combobox part
- [x] ContextMenuRoot — base.ContextMenu
- [x] ContextMenuTrigger — base.ContextMenu
- [x] DialogBackdrop — base.Dialog
- [x] DialogClose — base.Dialog
- [x] DialogDescription — base.Dialog
- [x] DialogPopup — base.Dialog
- [x] DialogPortal — base.Dialog
- [x] DialogRoot — base.Dialog
- [x] DialogTitle — base.Dialog
- [x] DialogTrigger — base.Dialog
- [x] DialogViewport — base.Dialog
- [x] DrawerBackdrop — base.Drawer
- [x] DrawerContent — base.Drawer
- [ ] DrawerIndent — Drawer part
- [ ] DrawerIndentBackground — Drawer part
- [x] DrawerPopup — base.Drawer
- [x] DrawerPortal — base.Drawer
- [ ] DrawerProvider — Drawer part
- [x] DrawerRoot — base.Drawer
- [ ] DrawerSwipeArea — Drawer part
- [x] DrawerViewport — base.Drawer
- [ ] FieldControl — Field part
- [x] FieldDescription — base.Field
- [x] FieldError — base.Field
- [ ] FieldItem — Field part
- [x] FieldLabel — base.Field
- [x] FieldRoot — base.Field
- [ ] FieldValidity — Field part
- [x] FieldsetLegend — base.Fieldset
- [x] FieldsetRoot — base.Fieldset
- [x] Form — base.Form
- [x] Input — base.Input
- [x] MenuArrow — base.Menu / base.Submenu / base.ContextMenu (`arrow` prop)
- [x] MenuBackdrop — base.Menu / base.Submenu / base.ContextMenu (`backdrop` prop)
- [x] MenuCheckboxItem — base.Menu
- [x] MenuCheckboxItemIndicator — base.Menu
- [x] MenuGroup — base.Menu
- [x] MenuGroupLabel — base.Menu
- [x] MenuItem — base.Menu
- [x] MenuLinkItem — base.Menu
- [x] MenuPopup — base.Menu
- [x] MenuPortal — base.Menu
- [x] MenuPositioner — base.Menu
- [x] MenuRadioGroup — base.Menu
- [x] MenuRadioItem — base.Menu
- [x] MenuRadioItemIndicator — base.Menu
- [x] MenuRoot — base.Menu
- [x] MenuSeparator — base.Menu
- [x] MenuSubmenuRoot — base.Menu
- [x] MenuSubmenuTrigger — base.Menu
- [x] MenuTrigger — base.Menu
- [x] Menubar — base.Menubar
- [x] MeterIndicator — base.Meter
- [x] MeterLabel — base.Meter
- [x] MeterRoot — base.Meter
- [x] MeterTrack — base.Meter
- [x] MeterValue — base.Meter
- [x] NavigationMenuArrow — base.NavigationMenu (`arrow` prop)
- [x] NavigationMenuBackdrop — base.NavigationMenu (`backdrop` prop)
- [x] NavigationMenuContent — base.NavigationMenu
- [ ] NavigationMenuIcon — NavigationMenu part
- [x] NavigationMenuItem — base.NavigationMenu
- [ ] NavigationMenuLink — NavigationMenu part
- [x] NavigationMenuList — base.NavigationMenu
- [x] NavigationMenuPopup — base.NavigationMenu
- [x] NavigationMenuPortal — base.NavigationMenu
- [x] NavigationMenuPositioner — base.NavigationMenu
- [x] NavigationMenuRoot — base.NavigationMenu
- [x] NavigationMenuTrigger — base.NavigationMenu
- [x] NavigationMenuViewport — base.NavigationMenu
- [x] NumberFieldDecrement — base.NumberField
- [x] NumberFieldGroup — base.NumberField
- [x] NumberFieldIncrement — base.NumberField
- [x] NumberFieldInput — base.NumberField
- [x] NumberFieldRoot — base.NumberField
- [ ] NumberFieldScrubArea — NumberField part
- [ ] NumberFieldScrubAreaCursor — NumberField part
- [x] OTPFieldInput — base.OTPField
- [x] OTPFieldRoot — base.OTPField
- [ ] OtpSlotElement — OtpField part
- [x] PopoverArrow — base.Popover (`arrow` prop)
- [x] PopoverBackdrop — base.Popover (`backdrop` prop)
- [x] PopoverClose — base.Popover (`close` prop)
- [x] PopoverDescription — base.Popover
- [x] PopoverPopup — base.Popover
- [x] PopoverPortal — base.Popover
- [x] PopoverPositioner — base.Popover
- [x] PopoverRoot — base.Popover
- [x] PopoverTitle — base.Popover
- [x] PopoverTrigger — base.Popover
- [ ] PopoverViewport — Popover part
- [x] PreviewCardArrow — base.PreviewCard (`arrow` prop)
- [x] PreviewCardBackdrop — base.PreviewCard (`backdrop` prop)
- [x] PreviewCardPopup — base.PreviewCard
- [x] PreviewCardPortal — base.PreviewCard
- [x] PreviewCardPositioner — base.PreviewCard
- [x] PreviewCardRoot — base.PreviewCard
- [x] PreviewCardTrigger — base.PreviewCard
- [x] PreviewCardViewport — base.PreviewCard
- [x] ProgressIndicator — base.Progress
- [x] ProgressLabel — base.Progress
- [x] ProgressRoot — base.Progress
- [x] ProgressTrack — base.Progress
- [x] ProgressValue — base.Progress
- [x] RadioGroupIndicator — base.RadioGroup
- [x] RadioGroupRadio — base.RadioGroup
- [x] RadioGroupRoot — base.RadioGroup
- [x] ScrollAreaContent — base.ScrollArea
- [x] ScrollAreaCorner — base.ScrollArea (`orientation: Both`)
- [x] ScrollAreaRoot — base.ScrollArea
- [x] ScrollAreaScrollbar — base.ScrollArea
- [x] ScrollAreaThumb — base.ScrollArea
- [x] ScrollAreaViewport — base.ScrollArea
- [x] SelectArrow — base.Select (`arrow` prop)
- [x] SelectBackdrop — base.Select (`backdrop` prop)
- [x] SelectGroup — base.OptionGroup
- [x] SelectGroupLabel — base.OptionGroup
- [x] SelectIcon — base.Select
- [x] SelectItem — base.Select
- [x] SelectItemIndicator — base.Select
- [x] SelectItemText — base.Select
- [x] SelectLabel — base.Select (`label` prop)
- [x] SelectList — base.Select
- [x] SelectPopup — base.Select
- [x] SelectPortal — base.Select
- [x] SelectPositioner — base.Select
- [x] SelectRoot — base.Select
- [ ] SelectScrollDownArrow — Select part
- [ ] SelectScrollUpArrow — Select part
- [x] SelectSeparator — base.OptionSeparator
- [x] SelectTrigger — base.Select
- [x] SelectValue — base.Select
- [x] Separator — base.Separator
- [x] SliderControl — base.Slider
- [x] SliderIndicator — base.Slider
- [x] SliderLabel — base.Slider (`label` prop)
- [x] SliderRoot — base.Slider
- [x] SliderThumb — base.Slider
- [x] SliderTrack — base.Slider
- [x] SliderValue — base.Slider (`show_value` prop)
- [x] SwitchRoot — base.Switch
- [x] SwitchThumb — base.Switch
- [x] TabsIndicator — base.Tabs
- [x] TabsList — base.Tabs
- [x] TabsPanel — base.Tabs
- [x] TabsRoot — base.Tabs
- [x] TabsTab — base.Tabs
- [ ] ToastAction — Toast part
- [ ] ToastClose — Toast part
- [ ] ToastContent — Toast part
- [ ] ToastDescription — Toast part
- [ ] ToastPortal — Toast part
- [ ] ToastProvider — Toast part
- [ ] ToastRoot — Toast part
- [ ] ToastTitle — Toast part
- [ ] ToastViewport — Toast part
- [x] Toggle — base.Toggle
- [x] ToggleGroup — base.ToggleGroup
- [x] ToolbarButton — base.Toolbar
- [x] ToolbarGroup — base.Toolbar
- [x] ToolbarInput — base.Toolbar
- [x] ToolbarLink — base.Toolbar
- [x] ToolbarRoot — base.Toolbar
- [x] ToolbarSeparator — base.Toolbar
- [x] TooltipPopup — base.Tooltip
- [x] TooltipPortal — base.Tooltip
- [x] TooltipPositioner — base.Tooltip
- [ ] TooltipProvider — Tooltip part
- [x] TooltipRoot — base.Tooltip
- [x] TooltipTrigger — base.Tooltip
- [ ] TooltipViewport — Tooltip part

Non-part exports — reviewed, not widgets:

- [ ] init — fn — registers every component's key bindings on the `App` — the adapter calls it on first render
- [ ] DirectionProvider — wraps a subtree with LTR/RTL context for arrow-key navigation — context, not a visual widget
- [ ] PresenceState — mount-presence state behind exit transitions — internal to popups
- [ ] SafePolygon — hover-intent geometry tracker behind Menu/NavigationMenu hover
- [ ] primitives.Input — shared text-edit element (`primitives::input::Input` + `InputTextElement`) behind Input and field inputs
- [ ] primitives.Scrollbar — shared scrollbar element (`primitives::scroll::Scrollbar`) behind ScrollArea and scrollable popups
- [ ] AlertDialogHandle — imperative open/close handle — not an element
- [ ] DialogHandle — imperative open/close handle — not an element
- [ ] TooltipHandle — imperative open/close handle — not an element
- [ ] SliderValues — value type — slider thumb values behind the Slider arm
- [ ] OTPFieldValidationType — enum — OTP input validation mode
- [ ] OTPNormalizedValue — value type behind OTPField normalize handlers

## video Components

docs: https://github.com/cijiugechu/gpui-video-player  
github: https://github.com/cijiugechu/gpui-video-player

> `use: video: gpui-video-player` — GStreamer 1.14+ required.

- [x] Player
- [ ] Video — engine handle (Arc<RwLock<Internal>>) owning the GStreamer pipeline — not an element
- [ ] VideoElement — needs a live `Video` handle + GStreamer runtime — can't be constructed declaratively

## apple-hig Components

docs: https://crates.io/crates/gpui-apple-hig  
github: https://crates.io/crates/gpui-apple-hig

> `use: hig: gpui-apple-hig` — rendered by `forge-lib-hig`. `[x]` = reachable as `hig.<note>` (forge element name).

- [ ] AppleAppearance — theme enum — light/dark, set on the window not an element
- [ ] AppleColors — semantic color token table — theme values, not widgets
- [ ] MAC_TRAFFIC_LIGHT_CLEARANCE — window-layout constant (px offset)
- [ ] MONO_FONT_NAME — font-name constant
- [ ] MacAccentColor — accent-color enum — set via `color`/`accent` props
- [x] MacActivityRings — hig.ActivityRings
- [x] MacBadge — hig.Badge
- [x] MacButton — hig.Button
- [x] MacChart — hig.Chart
- [ ] MacChartDataPoint — chart point value type — built from `points` props
- [x] MacCheckbox — hig.Checkbox
- [ ] MacColumnData — column-view column value type — built from `columns` props
- [x] MacColumnView — hig.ColumnView
- [x] MacContextMenu — hig.ContextMenu
- [x] MacDatePicker — hig.DatePicker
- [x] MacDialog — hig.Dialog
- [x] MacGauge — hig.Gauge
- [x] MacImageView — hig.ImageView
- [x] MacKbd — hig.Kbd
- [x] MacMenuBar — hig.MenuBar
- [ ] MacMenuBarMenu — menu name descriptor — built from `menus` props
- [ ] MacMenuItem — menu item descriptor — built from `items` props
- [ ] MacOutlineItem — outline row descriptor — built from `items` props
- [x] MacOutlineView — hig.OutlineView
- [x] MacPanel — hig.Panel
- [x] MacPathControl — hig.PathControl
- [ ] MacPathItem — path segment descriptor — built from `items` props
- [x] MacPopover — hig.Popover
- [x] MacPopupButton — hig.PopupButton
- [x] MacProgress — hig.Progress
- [x] MacPullDownButton — hig.PullDownButton
- [ ] MacRingMetric — ring value type — built from `rings` props
- [ ] MacSegment — segment descriptor — built from `items` props
- [x] MacSegmentedControl — hig.SegmentedControl
- [x] MacSettingsGroup — hig.SettingsGroup
- [ ] MacSettingsRow — settings row descriptor — children of SettingsGroup build it
- [x] MacSidebar — hig.Sidebar
- [ ] MacSidebarItem — sidebar row descriptor — built from `items`/children
- [ ] MacSidebarSection — sidebar section descriptor — children group items
- [x] MacSpinner — hig.Spinner
- [x] MacSwitch — hig.Switch
- [x] MacTabBar — hig.TabBar
- [ ] MacTabItem — tab descriptor — built from `items`/children props
- [ ] MacTableColumn — column descriptor — built from `columns` props
- [ ] MacTableRow — row value type — built from `rows` props
- [x] MacTableView — hig.TableView
- [x] MacToolbar — hig.Toolbar
- [ ] SYSTEM_FONT_NAME — font-name constant

## pdf Components

docs: https://packetthrower.github.io/zorite/reference/crates/gpui-pdf/  
github: https://github.com/packetThrower/zorite

> `use: pdf: gpui-pdf` — opt-in zorite adapter (`forge-lib-zorite` `pdf` feature).

- [x] Viewer
- [ ] LinkTarget — link destination value type for PdfView
- [ ] NormPoint — normalized-coordinate point for hit testing
- [ ] NormRect — normalized-coordinate rect for hit testing
- [ ] OutlineItem — document-outline node consumed by PdfView
- [ ] PageText — extracted page text value type
- [ ] PdfLink — link annotation value type

## whiteboard Components

docs: https://packetthrower.github.io/zorite/  
github: https://github.com/packetThrower/zorite

> `use: wb: gpui-whiteboard` — opt-in zorite adapter.

- [x] Board

## ratex Components

docs: https://packetthrower.github.io/zorite/  
github: https://github.com/packetThrower/zorite

> `use: math: ratex-gpui` — TeX math rendering, opt-in zorite adapter.

- [x] Tex
- [x] Editor
- [ ] MathEditor — `impl Render` entity — needs Entity<MathEditor> + keymap/editor setup
- [ ] MathNav — enum — editor navigation ops

## rhythm Components

docs: https://crates.io/crates/rhythm-gpui  
github: https://crates.io/crates/rhythm-gpui

> `use: rhythm: rhythm-gpui` (guise feature-gated adapter).

- [x] Frame
- [ ] DropCapRhythm — typography metric struct, not a widget
- [ ] FontRhythm — typography metric struct, not a widget
- [ ] Rhythm — typography metric struct, not a widget

## gpui-symbols Components

docs: https://gpui.dev  
github: https://github.com/AprilNEA/gpui-symbols

> `use: symbols: gpui-symbols` (guise feature-gated adapter).

- [x] Icon
- [ ] SfSymbol — rasterizer — `render()` yields RenderImage. `Icon` wraps it
- [ ] SymbolScale — enum — small/medium/large for SfSymbol
- [ ] SymbolWeight — enum — stroke weight for SfSymbol

## ratatui Components

docs: https://ratatui.rs/  
github: https://github.com/ratatui/ratatui

> ratatui-flavor base — the catalog mixes core `ratatui::widgets` arms,
> Forge layout primitives, and extension crates from the awesome-ratatui
> ecosystem. `[x]` entries render in the ratatui arm and on the gpui compat
> canvas via `canvas:` mappings. Extension widgets link their backing
> crate. Native-only entries pull `crossterm` or C sources and fall back to the
> placeholder on the WASM canvas.

Core `ratatui::widgets` coverage:

- [x] BarChart — `data`/`labels`/`bar_width`/`max`
- [ ] Bar — bar value type consumed by BarChart
- [ ] BarGroup — grouping descriptor for BarChart
- [x] Block
- [x] Calendar — `Monthly` behind `widget-calendar`. `date` prop
- [x] Canvas — `map` draws the world map layer
- [x] Chart — `data`/`name`/axis bounds
- [ ] Circle — canvas shape — needs a Canvas draw context
- [x] Clear — clears its rect
- [ ] Line — canvas shape — needs a Canvas draw context
- [x] Gauge
- [x] LineGauge — `value`/`label`
- [x] List
- [ ] ListItem — row value type for `List`
- [x] Logo — `RatatuiLogo` `tiny`|`small`
- [ ] Map — reached via `Canvas`'s `map` prop, not a standalone element
- [x] Mascot — `RatatuiMascot`
- [x] Paragraph — rendered as `Text`/`Title`
- [ ] Points — canvas shape — needs a Canvas draw context
- [ ] Rectangle — canvas shape — needs a Canvas draw context
- [ ] Row — row descriptor for `Table`
- [ ] Cell — cell descriptor inside `Row`
- [x] Scrollbar — bound `position` takes wheel and track clicks
- [x] Sparkline — `data`/`max`
- [x] Table
- [x] Tabs
- [ ] World — map dataset for the canvas `Map` layer

Forge layout and form primitives (no upstream analogue):

- [x] VStack — vertical layout
- [x] HStack — horizontal layout
- [x] Spacer — empty area, sized by style
- [x] Text — styled paragraph
- [x] Title — bold text
- [x] Divider — horizontal rule
- [x] Log — scrollable text, bound `offset`
- [x] Paper — `Block` alias
- [x] Card — `Block` alias
- [x] Progress — `Gauge` alias
- [x] Input — single-line edit, bound `value`
- [x] TextInput — `Input` alias
- [x] Button — `click` call on Enter/Space/down
- [x] Select — bound `selected` list
- [x] Menu — `Select` alias
- [x] Checkbox — bound `checked`
- [x] Switch — `Checkbox` alias
- [x] Fill — one glyph tiled over the rect

Extension widgets — awesome-ratatui crates on the 0.30 line:

- [x] Tree (https://crates.io/crates/tui-tree-widget) — `items` or `TreeItem` children, `selected`/`expanded`
- [x] TreeItem — `Tree` child — `id` + nested `TreeItem`s
- [x] Spinner (https://crates.io/crates/throbber-widgets-tui) — all symbol sets, `step` binds the frame
- [x] Markdown (https://crates.io/crates/ratatui-markdown) — native-only (ratatui crossterm default)
- [x] AnsiText (https://crates.io/crates/ansi-to-tui) — ANSI escapes → styled spans
- [x] Terminal — ANSI screen → TestBackend grid, `offset` scrollback
- [x] ScrollView (https://crates.io/crates/tui-scrollview) — virtual `width`x`height`, bound `offset`
- [x] CodeEditor (https://crates.io/crates/ratatui-code-editor) — tree-sitter highlight, `original` diff — native-only (C grammars)
- [x] Toast (https://crates.io/crates/ratatui-toaster) — `type` tints
- [x] Popup (https://crates.io/crates/tui-popup) — children float centered, `width`/`height`
- [x] WidgetList (https://crates.io/crates/tui-widget-list) — mixed-height items, bound `selected`
- [x] Nodes (https://crates.io/crates/tui-nodes) — `edges` port links, auto layout
- [x] GraphNode — `Nodes` child box — `width`/`height`
- [x] Shimmer (https://crates.io/crates/tui-shimmer) — sweep highlight, `phase` 0..1
- [x] Dialog (https://crates.io/crates/tui_confirm_dialog) — `open`/`choice` bindings
- [x] Fx (https://crates.io/crates/tachyonfx) — `kind`/`duration`/`progress` cell post-effects
- [x] QrCode (https://crates.io/crates/qrcode) — unicode halfblocks from `data`
- [x] Image (https://crates.io/crates/image) — decode → halfblock raster, `src` on `base_dir`
- [x] Slider (https://crates.io/crates/tui-slider) — bound `value` keys/wheel/clicks — native-only (crossterm)
- [x] PieChart (https://crates.io/crates/tui-piechart) — `data`/`labels`/`hole` — native-only (crossterm)
- [x] BigText (https://crates.io/crates/tui-big-text) — 8x8 pixel font, `size` density — native-only (crossterm)
- [x] Globe (https://crates.io/crates/tui-globe) — embedded mesh, bound `yaw` rotates — native-only (crossterm)

## iced Components

docs: https://iced.rs/  
github: https://github.com/iced-rs/iced

> iced-flavor base — `[x]` widgets render via `canvas:` mappings (`Input`↔`TextInput`, `Switch`↔`Toggler`, `Progress`↔`ProgressBar`, `VStack`/`HStack`↔`Column`/`Row`).

- [x] Button
- [x] Canvas
- [x] Checkbox
- [x] Column
- [x] ComboBox
- [x] Container
- [ ] Float — widget wrapper needing an anchor+overlay callback
- [x] Image
- [x] KeyedColumn
- [ ] Lazy — widget wrapper keyed on a caller-supplied closure
- [x] Markdown
- [x] MouseArea — iced arm — child wrapper + click hooks
- [x] PaneGrid
- [x] PickList
- [ ] Pin — widget wrapper anchoring children — Elm overlay semantics
- [x] ProgressBar
- [ ] QRCode — behind iced's `qr_code` cargo feature, not enabled on the runtime dep
- [x] Radio
- [ ] Responsive — generic viewport switch — takes a layout closure, not props
- [x] Row
- [x] Rule
- [x] Scrollable
- [ ] Sensor — widget wrapper emitting window/resize sensor events
- [ ] Shader — widget wrapper taking a wgpu shader Program
- [x] Slider
- [x] Space
- [x] Stack
- [x] Svg
- [x] Table
- [x] Text
- [x] TextEditor
- [x] TextInput
- [x] Toggler
- [x] Tooltip — iced arm — child + `tip`/`position` props
- [x] VerticalSlider

## bezel Components

docs: https://crates.io/crates/bezel  
github: https://crates.io/crates/bezel

> The `bezel` base catalog renders via `canvas:` mappings. The `bezel-*` crates are scaffolds with no Forge adapter yet.

- [x] VStack — → VStack
- [x] HStack — → HStack
- [x] Text — → Text
- [x] Button — → Button
- [x] Input — → Input
- [x] Card — → GroupBox
- [ ] bezel-canvas.Canvas — scaffold — no adapter
- [ ] bezel-chart.Chart — scaffold — no adapter
- [ ] bezel-editor.Editor — scaffold — no adapter
- [ ] bezel-icons.Icon — scaffold — no adapter
- [ ] bezel-markdown.Markdown — scaffold — no adapter
- [ ] bezel-motion.Fade — scaffold — no adapter
- [ ] bezel-terminal.Terminal — scaffold — no adapter

## adabraka Components

docs: https://augani.github.io/adabraka-ui/  
github: https://crates.io/crates/adabraka-ui

> adabraka-ui (shadcn flavor) on the adabraka-gpui fork — the catalog's `[x]` elements render via `canvas:`/shared arms. The remaining upstream exports have no Forge adapter yet.

- [x] VStack — → VStack
- [x] HStack — → HStack
- [x] Card — → GroupBox
- [x] Text — → Text
- [x] Badge — → shared arm
- [x] Avatar — → Avatar
- [x] Button — → Button
- [x] Input — → Input
- [x] Checkbox — → Checkbox
- [x] Switch — → Switch
- [x] Select — → Select
- [x] Progress — → Progress
- [x] Alert — → shared arm
- [x] Dialog — → Dialog
- [x] Accordion
- [ ] AccordionItem — item descriptor for `Accordion`
- [x] AlertDialog
- [x] AnimatedCounter — cataloged as `AnimatedCounter` (canvas: mapping)
- [x] AnimatedList — cataloged as `AnimatedList` (canvas: mapping)
- [x] AnimatedPresence — cataloged as `AnimatedPresence` (canvas: mapping)
- [ ] AnimatedSwitch — implemented as `Switch` (component path maps to animated_switch::AnimatedSwitch)
- [x] AnimatedText — cataloged as `AnimatedText` (canvas: mapping)
- [x] AreaChart
- [ ] AreaChartSeries — series value type for `AreaChart`
- [x] AvatarGroup
- [ ] AvatarItem — row value type for `AvatarGroup`
- [x] BarChart
- [ ] BarChartData — data value type for `BarChart`
- [ ] BarChartSeries — series value type for `BarChart`
- [x] BottomSheet
- [ ] BreadcrumbItem — item descriptor for `Breadcrumbs`
- [x] Breadcrumbs
- [x] Calendar
- [x] ColorPicker
- [ ] ColumnDef — column descriptor for `DataTable`
- [x] Combobox
- [ ] Command — palette item descriptor for `CommandPalette`
- [x] Confetti — cataloged as `Confetti` (canvas: mapping)
- [x] ContextMenu
- [ ] ContextMenuItem — menu item descriptor for `ContextMenu`
- [x] CopyButton
- [x] CropArea — cataloged as `CropArea` (canvas: mapping)
- [x] DataTable
- [ ] DateFormat — format enum for `DatePicker`
- [x] DatePicker
- [x] Dock — cataloged as `Dock` (canvas: mapping)
- [x] DonutChart
- [ ] DragData — payload wrapper behind DropZone/Draggable
- [ ] Draggable — drag-source primitive — `SortableList` covers reorderable lists
- [x] DrawerNavigation — cataloged as `DrawerNavigation` (canvas: mapping)
- [ ] DrawerSide — enum — drawer anchor side
- [x] DropZone — cataloged as `DropZone` (canvas: mapping)
- [x] Dropdown
- [ ] DropdownItem — item descriptor for `Dropdown`
- [x] Editor
- [ ] EditorLanguage — prelude alias for editor `Language` enum
- [x] ExpandableCard
- [ ] FileNode — node value type for `FileTree`
- [x] FileTree
- [x] FloatingActionButton
- [x] Form
- [x] Gauge
- [x] GlassMorphism — cataloged as `GlassMorphism` (canvas: mapping)
- [x] HotkeyInput — cataloged as `HotkeyInput` (canvas: mapping)
- [x] HoverCard
- [x] InfiniteScroll
- [x] InlineEdit
- [x] KBD
- [ ] LabelSide — enum — toggle label side
- [ ] LayoutAnimation — enum for layout-transition presets
- [x] LineChart — cataloged as `LineChartAda` (canvas: mapping)
- [ ] LineChartPoint — point value type for LineChart
- [ ] LineChartSeries — series value type for LineChart
- [x] MagneticButton — cataloged as `MagneticButton` (canvas: mapping)
- [x] Marquee — cataloged as `Marquee` (canvas: mapping)
- [x] Meteors — cataloged as `Meteors` (canvas: mapping)
- [x] NavigationMenu — cataloged as `NavigationMenuWidget` (canvas: mapping)
- [x] NavigationMenuItem — base.NavigationMenu
- [x] NumberInput — cataloged as `NumberInputField` (canvas: mapping)
- [x] PopoverMenu — cataloged as `PopoverMenu` (canvas: mapping)
- [ ] PopoverMenuItem — item descriptor for PopoverMenu
- [x] RadarChart — cataloged as `RadarChartAda` (canvas: mapping)
- [ ] RadarDataset — dataset value type for RadarChart
- [x] Radio — cataloged as `Radio` (canvas: mapping)
- [x] RadioGroup
- [ ] RadioLayout — enum — radio group layout
- [x] RangeSlider — cataloged as `RangeSlider` (canvas: mapping)
- [x] Rating
- [x] ResizablePanel — cataloged as `ResizablePanel` (canvas: mapping)
- [x] ResizablePanelGroup — cataloged as `ResizablePanelGroup` (canvas: mapping)
- [ ] RichBlock — rich-text block AST for RichText
- [ ] RichInline — rich-text inline AST for RichText
- [x] SearchInput — cataloged as `SearchInput` (canvas: mapping)
- [x] SegmentedNav — cataloged as `SegmentedNav` (canvas: mapping)
- [ ] SelectOption — option descriptor for `Select`/`Combobox`
- [x] Separator — cataloged as `Separator` (canvas: mapping)
- [x] Sheet
- [ ] SheetSide — enum — sheet anchor edge
- [x] Skeleton
- [x] Slider
- [ ] SliderAxis — enum — slider orientation
- [x] SortableList — cataloged as `SortableList` (canvas: mapping)
- [x] Spinner — cataloged as `Spinner` (canvas: mapping)
- [x] Spotlight — cataloged as `Spotlight` (canvas: mapping)
- [x] StatusBar — cataloged as `StatusBarWidget` (canvas: mapping)
- [ ] StatusItem — item descriptor for StatusBar
- [ ] TabItem — item descriptor for `Tabs`
- [x] Table
- [ ] TableColumn — column descriptor for `Table`
- [ ] TableRow — row descriptor for `Table`
- [x] Tabs
- [x] TagInput — cataloged as `TagInput` (canvas: mapping)
- [ ] TextAnimation — animation enum for `AnimatedText`
- [x] TextReveal — cataloged as `TextReveal` (canvas: mapping)
- [ ] ThemeTokens — design-token struct consumed by the theme system
- [x] TiltCard — cataloged as `TiltCard` (canvas: mapping)
- [ ] ToastItem — toast descriptor consumed by ToastManager
- [ ] ToastManager — overlay host entity (Render, not RenderOnce) — mounted once at app root
- [x] Toggle
- [x] TreeList — cataloged as `TreeList` (canvas: mapping)
- [ ] TreeNode — node value type for TreeList/FileTree
- [x] TypeWriter — cataloged as `TypeWriter` (canvas: mapping)
- [x] ViewRouter — cataloged as `ViewRouter` (canvas: mapping)

## forge built-ins

docs: https://gpui-forge.rudeboy.dev  
github: https://github.com/

> `forge.*` — Forge tooling elements rendered by the runtime itself.

- [x] Preview
- [x] Outline
- [x] Inspector
- [x] InspectorField
- [x] Icons
- [x] Files
- [x] Palette
- [x] Showcase
- [x] Vars

## template-lib Components

docs: https://crates.io/crates/template-lib  
github: https://crates.io/crates/template-lib

> Template adapter scaffold (`forge-lib-template`).

- [x] Pill
