# All Components

Every exported element/widget in each upstream library — `[x]` = implemented for Forge
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
- [ ] Menu (https://gpui.rs/components/menu) — native app menus are host-level; in-doc menus: ContextMenu/DropdownMenu
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

## GPUI-KIT upstream extras (gpui-component crate)

docs: https://gpui-kit.com/  
github: https://github.com/longbridge/gpui-kit

> Exported by the upstream component crate but not yet in the Forge catalog.

- [ ] Cancel
- [ ] Confirm
- [ ] Date — value type, not a widget — `DatePicker`/`Calendar` produce it
- [ ] DiffConflict
- [ ] DiffConflictResolution
- [ ] DiffFile
- [ ] DiffFileStatus
- [ ] DiffSide
- [ ] Disableable
- [ ] History
- [ ] HourCycle
- [ ] Matcher
- [ ] NumberStep
- [ ] PlotHover
- [ ] PopupMenu
- [ ] PopupMenuItem
- [ ] RadarChart
- [ ] RadarLabel
- [ ] RoleOverride
- [ ] Rope
- [ ] RopeLines
- [ ] SankeyChart
- [ ] SankeyLabel
- [ ] SearchableGroup
- [ ] SearchableListDelegate
- [ ] SearchableListItem
- [ ] SearchableVec
- [ ] Selectable
- [ ] Sizable
- [ ] SliderScale
- [ ] SpeechStatus
- [ ] StyleSized
- [ ] TimePrecision
- [ ] TimeSegment
- [ ] UndoHistory
- [ ] WindowBorder

## ely Components

docs: https://elygpui.com/  
github: https://github.com/ELY-Open-Source/Ely-GPUI-Components

> The ely site advertises 1279 items (widgets + ~900 icons). Below is every exported widget type in `ely-core` 0.2 — the `[x]` entries render natively via `forge-lib-ely` (`use: ely: ely-core`); the rest have no Forge adapter support yet.

- [ ] AboutDialog
- [ ] Accordion
- [ ] AccordionItem
- [ ] ActionBar
- [x] Activity
- [ ] ActivityBar
- [x] ActivityFeed
- [ ] ActivityItem
- [ ] Aggregate
- [x] Alert
- [ ] AlertDialog
- [ ] Anchor
- [ ] AnimatePresence
- [ ] AnimatedGradient
- [ ] AppShell
- [x] AspectRatio
- [ ] AsyncView
- [x] Avatar
- [x] AvatarGroup
- [ ] Backdrop
- [x] Badge
- [x] Banner
- [x] Barcode
- [x] BeforeAfter
- [ ] Blink
- [ ] Blockquote
- [ ] Breadcrumb
- [ ] BulkActionBar
- [x] Button
- [ ] ButtonGroup
- [ ] COUNTRIES
- [ ] Calendar
- [x] Callout
- [x] Caption
- [ ] Card
- [ ] CardHeader
- [x] Carousel
- [ ] Cascade
- [ ] Cascader
- [ ] Cell
- [ ] Change
- [ ] Changeable
- [x] Changelog
- [x] Checkbox
- [ ] CheckboxCard
- [ ] CheckboxGroup
- [ ] Close
- [x] Code
- [x] Collapsible
- [ ] ColorSwatch
- [ ] Column
- [ ] ColumnShell
- [ ] Command
- [x] Comparison
- [ ] Confetti
- [ ] ConfirmButton
- [ ] ConfirmDialog
- [ ] ConnectionStatus
- [ ] Connectivity
- [x] Container
- [ ] ContextMenu
- [ ] Copy
- [ ] Corner
- [x] CountBadge
- [ ] Countdown
- [ ] Crumb
- [ ] Cut
- [ ] DataGrid
- [ ] DataTable
- [ ] DatePicker
- [ ] DateRangePicker
- [ ] DateTimePicker
- [x] DescriptionList
- [ ] Dialog
- [ ] DirectoryListing
- [ ] Disableable
- [ ] Dock
- [ ] DockLayout
- [ ] DockPanel
- [ ] DockSide
- [x] DotBadge
- [ ] Drawer
- [ ] Drawing
- [ ] DropAt
- [ ] DropZone
- [ ] Dropdown
- [ ] DropdownMenu
- [ ] DurationPicker
- [ ] Editing
- [ ] EditorTab
- [ ] EditorTabs
- [ ] Ellipsis
- [ ] EllipsisTooltip
- [ ] EmailInput
- [ ] EmojiPicker
- [ ] Entrance
- [ ] ErrorBoundary
- [ ] ErrorView
- [ ] ExpressionInput
- [ ] ExternalLink
- [ ] FieldArray
- [x] Fieldset
- [ ] FileInput
- [ ] FileTree
- [ ] Finder
- [ ] Flash
- [ ] Flip
- [ ] FloatingToolbar
- [ ] Folder
- [ ] FontPicker
- [ ] Frame
- [x] Gallery
- [x] Gauge
- [ ] Glow
- [ ] GradientEditor
- [ ] GradientText
- [ ] Grid
- [ ] HUE_NAMES
- [x] Heading
- [ ] HeatmapTable
- [ ] HelpTooltip
- [ ] Highlight
- [ ] History
- [ ] Hosted
- [x] IconButton
- [ ] IconPicker
- [ ] Image
- [ ] InfiniteList
- [x] InlineEdit
- [x] InlineMessage
- [ ] Input
- [ ] InputAddon
- [ ] InputGroup
- [ ] IntersectionObserver
- [ ] Kbd
- [ ] KbdCombo
- [ ] KeyValueInput
- [x] KpiCard
- [ ] LEADING
- [ ] LOCALES
- [x] Label
- [ ] LazyLoad
- [ ] Lightbox
- [x] Link
- [ ] List
- [ ] ListInput
- [x] ListItem
- [ ] LoadMore
- [ ] LoadingOverlay
- [ ] Marquee
- [ ] MaskedInput
- [ ] Masonry
- [ ] MasterDetail
- [ ] Measure
- [ ] Menu
- [ ] MenuItem
- [x] Meter
- [ ] MiddleEllipsis
- [ ] MiniWindow
- [ ] Mix
- [ ] MonthPicker
- [ ] NavGroup
- [x] NavItem
- [x] Notification
- [ ] NotificationCenter
- [x] NumberInput
- [ ] OfflineIndicator
- [ ] OnClimb
- [ ] OnFold
- [ ] OverflowMenu
- [x] Overline
- [ ] PICTURES
- [ ] Page
- [x] Pagination
- [ ] PaneGroup
- [ ] PaneId
- [ ] PaneLayout
- [x] Panel
- [x] Paragraph
- [ ] ParticleBackground
- [ ] PasswordInput
- [ ] Paste
- [ ] PathInput
- [ ] Peek
- [ ] PhoneInput
- [ ] Pick
- [ ] PieItem
- [ ] PieMenu
- [ ] Place
- [ ] Presence
- [x] ProgressBar
- [ ] ProgressRing
- [ ] PromptDialog
- [x] PropertyGrid
- [x] PropertyGroup
- [ ] Pulse
- [x] QrCode
- [ ] QuarterPicker
- [ ] QuickOpen
- [ ] QuickSwitcher
- [x] Radio
- [x] RadioCard
- [ ] RadioGroup
- [ ] Raised
- [ ] RangeSlider
- [ ] Refresh
- [ ] RefreshIndicator
- [ ] RegexInput
- [ ] RelativeDatePicker
- [x] Release
- [ ] Ripple
- [ ] Row
- [ ] SavingIndicator
- [ ] ScrollModel
- [ ] ScrollPlace
- [ ] ScrollShadow
- [ ] ScrollToTop
- [ ] ScrubInput
- [ ] SearchableMenu
- [x] Section
- [ ] Sections
- [ ] SelectAll
- [ ] Selectable
- [ ] SelectableList
- [ ] Shake
- [ ] Sheet
- [ ] ShellColumn
- [ ] Shimmer
- [ ] ShimmerText
- [ ] SignaturePad
- [ ] SimpleGrid
- [ ] Sizable
- [x] Skeleton
- [ ] SkeletonAvatar
- [ ] SkeletonCard
- [ ] SkeletonTable
- [ ] SkeletonText
- [x] Slider
- [ ] SortKey
- [x] Sparkline
- [x] Spinner
- [ ] SplashScreen
- [ ] SplitButton
- [ ] SplitPane
- [ ] Spot
- [ ] Spotlight
- [ ] Spreadsheet
- [ ] Stagger
- [x] Stars
- [x] Statistic
- [ ] StatusBar
- [ ] StatusBarItem
- [x] StatusMessage
- [ ] Steps
- [ ] StickyHeader
- [ ] Stroke
- [x] Subtitle
- [ ] Suspense
- [ ] SwipeableListItem
- [ ] SyncStatus
- [ ] Syntax
- [ ] TabBar
- [ ] TabPlacement
- [ ] Table
- [ ] TableOfContents
- [ ] Tabs
- [x] Tag
- [ ] Test
- [ ] TextInput
- [ ] TimePicker
- [x] Timeline
- [x] TimelineItem
- [x] Title
- [ ] TitleBar
- [ ] Toast
- [ ] ToastViewport
- [ ] Toaster
- [x] ToggleButton
- [ ] ToggleGroup
- [ ] ToggleItem
- [ ] Tone
- [x] Toolbar
- [ ] ToolbarGroup
- [ ] ToolbarSeparator
- [ ] Tooltip
- [ ] Tour
- [ ] Transform
- [ ] Tree
- [ ] TreeNode
- [ ] TreeRow
- [ ] TreeSelect
- [ ] TreeTable
- [x] TrendIndicator
- [ ] Typewriter
- [ ] TypingIndicator
- [ ] UpdateBanner
- [ ] UpdateDialog
- [ ] Upload
- [ ] UploadList
- [ ] UrlInput
- [x] UsageBar
- [x] UserChip
- [ ] Viewport
- [ ] VirtualList
- [ ] WORKSPACE_VERSION
- [x] Watermark
- [ ] Well
- [ ] WindowControls
- [ ] WindowManager
- [ ] WindowTab
- [ ] Wizard
- [ ] Workspace
- [ ] YearPicker
- [x] ZStack
- [ ] ZoomControl
- [ ] ZoomStep
- [x] Divider
- [x] CopyButton
- [x] SegmentedControl
- [x] EmptyState
- [x] Switch
- [x] Rating
- [x] Stepper
- [x] Icon


## guise Components

docs: https://docs.rs/guise  
github: https://github.com/wess/guise

> Vanilla-gpui flavor base — `base: guise` on crates.io `gpui 0.2.2`. Below is every exported widget type in `guise-ui` 1.10 — `[x]` entries are mapped by `catalogs/guise.catalog.yaml` (`Stack`/`Group` surface as Forge `VStack`/`HStack`); each unchecked entry carries a one-line reason it stays unmapped.

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
- [ ] DropTarget — drop-target primitive — needs host drag state; `Dropzone` covers file drops
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
- [ ] ActiveNodeDrag
- [ ] Command
- [ ] CompositeCommand
- [ ] ContextMenuCanvasExtra
- [ ] CreateEdge
- [ ] CreatePort
- [ ] GraphChange
- [ ] GraphOp
- [ ] HistoryProvider
- [ ] LocalHistory
- [ ] NODE_DRAG_TICK_INTERVAL
- [ ] NodeRenderer
- [ ] ToastLevel
- [ ] ToastMessage

## base-gpui Components

docs: https://github.com/LukeTandjung/base-gpui  
github: https://github.com/LukeTandjung/base-gpui

> Vendored Zed `gpui` engine fork — `use: base: base-gpui`, rendered by `forge-lib-base`. The crate also exports the full gpui element/element-style API (engine primitives — not a widget set).

- [x] Button
- [x] Checkbox
- [x] Switch
- [x] Toggle
- [x] Slider
- [x] Progress
- [x] Meter
- [x] Separator
- [x] Input
- [x] Avatar

## video Components

docs: https://github.com/cijiugechu/gpui-video-player  
github: https://github.com/cijiugechu/gpui-video-player

> `use: video: gpui-video-player` — GStreamer 1.14+ required.

- [x] Player
- [ ] Video
- [ ] VideoElement

## apple-hig Components

docs: https://crates.io/crates/gpui-apple-hig  
github: https://crates.io/crates/gpui-apple-hig

> `use: hig: gpui-apple-hig` — rendered by `forge-lib-hig`. `[x]` = reachable as `hig.<note>` (forge element name).

- [ ] AppleAppearance
- [ ] AppleColors
- [ ] MAC_TRAFFIC_LIGHT_CLEARANCE
- [ ] MONO_FONT_NAME
- [ ] MacAccentColor
- [ ] MacActivityRings
- [x] MacBadge — hig.Badge
- [x] MacButton — hig.Button
- [ ] MacChart
- [ ] MacChartDataPoint
- [x] MacCheckbox — hig.Checkbox
- [ ] MacColumnData
- [ ] MacColumnView
- [ ] MacContextMenu
- [ ] MacDatePicker
- [ ] MacDialog
- [ ] MacGauge
- [ ] MacImageView
- [ ] MacKbd
- [ ] MacMenuBar
- [ ] MacMenuBarMenu
- [ ] MacMenuItem
- [ ] MacOutlineItem
- [ ] MacOutlineView
- [ ] MacPanel
- [ ] MacPathControl
- [ ] MacPathItem
- [ ] MacPopover
- [ ] MacPopupButton
- [x] MacProgress — hig.Progress
- [ ] MacPullDownButton
- [ ] MacRingMetric
- [ ] MacSegment
- [ ] MacSegmentedControl
- [ ] MacSettingsGroup
- [ ] MacSettingsRow
- [ ] MacSidebar
- [ ] MacSidebarItem
- [ ] MacSidebarSection
- [x] MacSpinner — hig.Spinner
- [x] MacSwitch — hig.Switch
- [ ] MacTabBar
- [ ] MacTabItem
- [ ] MacTableColumn
- [ ] MacTableRow
- [ ] MacTableView
- [ ] MacToolbar
- [ ] SYSTEM_FONT_NAME

## pdf Components

docs: https://packetthrower.github.io/zorite/reference/crates/gpui-pdf/  
github: https://github.com/packetThrower/zorite

> `use: pdf: gpui-pdf` — opt-in zorite adapter (`forge-lib-zorite` `pdf` feature).

- [x] Viewer
- [ ] LinkTarget
- [ ] NormPoint
- [ ] NormRect
- [ ] OutlineItem
- [ ] PageText
- [ ] PdfLink

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
- [ ] MathEditor
- [ ] MathNav

## rhythm Components

docs: https://crates.io/crates/rhythm-gpui  
github: https://crates.io/crates/rhythm-gpui

> `use: rhythm: rhythm-gpui` (guise feature-gated adapter).

- [x] Frame
- [ ] DropCapRhythm
- [ ] FontRhythm
- [ ] Rhythm

## gpui-symbols Components

docs: https://gpui.dev  
github: https://github.com/AprilNEA/gpui-symbols

> `use: symbols: gpui-symbols` (guise feature-gated adapter).

- [x] Icon
- [ ] SfSymbol
- [ ] SymbolScale
- [ ] SymbolWeight

## ratatui Components

docs: https://ratatui.rs/  
github: https://github.com/ratatui/ratatui

> ratatui-flavor base — `[x]` widgets render via `canvas:` mappings; the catalog also provides `VStack`/`HStack`/`Input` analogues ratatui lacks.

- [x] BarChart
- [ ] Bar
- [ ] BarGroup
- [x] Block
- [x] Calendar
- [x] Canvas
- [x] Chart
- [ ] Circle
- [ ] Clear
- [ ] Fill
- [x] Gauge
- [ ] Line
- [x] List
- [ ] ListItem
- [x] Logo
- [ ] Map
- [x] Mascot
- [x] Paragraph
- [ ] Points
- [ ] Rectangle
- [ ] Row
- [ ] Cell
- [ ] Scrollbar — internal part of `ScrollArea`
- [x] Sparkline
- [x] Table
- [x] Tabs
- [ ] World

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
- [ ] Float
- [x] Image
- [x] KeyedColumn
- [ ] Lazy
- [x] Markdown
- [ ] MouseArea
- [x] PaneGrid
- [x] PickList
- [ ] Pin
- [x] ProgressBar
- [ ] QRCode
- [x] Radio
- [ ] Responsive — generic viewport switch — takes a layout closure, not props
- [x] Row
- [x] Rule
- [x] Scrollable
- [ ] Sensor
- [ ] Shader
- [x] Slider
- [x] Space
- [x] Stack
- [x] Svg
- [x] Table
- [x] Text
- [x] TextEditor
- [x] TextInput
- [x] Toggler
- [ ] Tooltip
- [x] VerticalSlider

## bezel Components

docs: https://crates.io/crates/bezel  
github: https://crates.io/crates/bezel

> The `bezel` base catalog renders via `canvas:` mappings; the `bezel-*` crates are scaffolds with no Forge adapter yet.

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

docs: https://crates.io/crates/adabraka-ui  
github: https://crates.io/crates/adabraka-ui

> adabraka-ui (shadcn flavor) on the adabraka-gpui fork — the catalog's `[x]` elements render via `canvas:`/shared arms; the remaining upstream exports have no Forge adapter yet.

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
- [ ] AccordionItem
- [x] AlertDialog
- [ ] AnimatedCounter
- [ ] AnimatedList
- [ ] AnimatedPresence
- [ ] AnimatedSwitch
- [ ] AnimatedText
- [x] AreaChart
- [ ] AreaChartSeries
- [x] AvatarGroup
- [ ] AvatarItem
- [x] BarChart
- [ ] BarChartData
- [ ] BarChartSeries
- [x] BottomSheet
- [ ] BreadcrumbItem
- [x] Breadcrumbs
- [x] Calendar
- [x] ColorPicker
- [ ] ColumnDef
- [x] Combobox
- [ ] Command
- [ ] Confetti
- [x] ContextMenu
- [ ] ContextMenuItem
- [x] CopyButton
- [ ] CropArea
- [x] DataTable
- [ ] DateFormat
- [x] DatePicker
- [ ] Dock
- [x] DonutChart
- [ ] DragData
- [ ] Draggable — drag-source primitive — `SortableList` covers reorderable lists
- [ ] DrawerNavigation
- [ ] DrawerSide
- [ ] DropZone
- [x] Dropdown
- [ ] DropdownItem
- [x] Editor
- [ ] EditorLanguage
- [x] ExpandableCard
- [ ] FileNode
- [x] FileTree
- [x] FloatingActionButton
- [x] Form
- [x] Gauge
- [ ] GlassMorphism
- [ ] HotkeyInput
- [x] HoverCard
- [x] InfiniteScroll
- [x] InlineEdit
- [x] KBD
- [ ] LabelSide
- [ ] LayoutAnimation
- [ ] LineChart
- [ ] LineChartPoint
- [ ] LineChartSeries
- [ ] MagneticButton
- [ ] Marquee
- [ ] Meteors
- [ ] NavigationMenu
- [ ] NavigationMenuItem
- [ ] NumberInput
- [ ] PopoverMenu
- [ ] PopoverMenuItem
- [ ] RadarChart
- [ ] RadarDataset
- [ ] Radio
- [x] RadioGroup
- [ ] RadioLayout
- [ ] RangeSlider
- [x] Rating
- [ ] ResizablePanel
- [ ] ResizablePanelGroup
- [ ] RichBlock
- [ ] RichInline
- [ ] SearchInput
- [ ] SegmentedNav
- [ ] SelectOption
- [ ] Separator
- [x] Sheet
- [ ] SheetSide
- [x] Skeleton
- [x] Slider
- [ ] SliderAxis
- [ ] SortableList
- [ ] Spinner
- [ ] Spotlight
- [ ] StatusBar
- [ ] StatusItem
- [ ] TabItem
- [x] Table
- [ ] TableColumn
- [ ] TableRow
- [x] Tabs
- [ ] TagInput
- [ ] TextAnimation
- [ ] TextReveal
- [ ] ThemeTokens
- [ ] TiltCard
- [ ] ToastItem
- [ ] ToastManager
- [x] Toggle
- [ ] TreeList
- [ ] TreeNode
- [ ] TypeWriter
- [ ] ViewRouter

## forge built-ins

docs: https://gpui-forge.rudeboy.dev  
github: https://github.com/

> `forge.*` — Forge tooling elements rendered by the runtime itself.

- [x] Preview
- [x] Outline
- [x] Inspector
- [x] Files
- [x] Palette
- [x] Showcase
- [x] Vars

## template-lib Components

docs: https://crates.io/crates/template-lib  
github: https://crates.io/crates/template-lib

> Template adapter scaffold (`forge-lib-template`).

- [x] Pill
