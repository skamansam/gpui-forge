# ratatui widget reference

Every element in `catalogs/ratatui.catalog.yaml`, grouped by source.
Docs declare the base with `base: ratatui`. Each element draws into a
ratatui cell grid. In Studio the canvas card renders the grid in-process
through `TestBackend`. On the WASM canvas the same renderer runs, but
native-only widgets fall back to a placeholder.

The full machine-readable list lives in
[ALL-COMPONENTS.md](../../ALL-COMPONENTS.md) and the generated
[components.md](components.md) index. Each extension widget links its
backing crate there.

## Style keys

All elements read these `style:` entries. Unknown keys are ignored.

| Key | Effect |
| --- | --- |
| `color` | Foreground. Named ANSI hue or `$theme.*` token |
| `background` | Background |
| `weight` | `bold` adds the bold modifier |
| `italic` | Italic modifier when truthy |
| `underline` | Underline modifier when truthy |
| `dim` | Dim modifier when truthy |
| `border-color` | Border foreground on `Block`, `Popup`, `Dialog` |
| `width`, `height`, `flex`, `min`, `max` | Layout constraints along the parent axis |

## Interaction model

Widgets take input through bound props. A bound prop makes the element
focusable. `Tab`/`Shift-Tab` cycles focusables in document order.

| Prop pattern | Input contract |
| --- | --- |
| `value: $x` | `Input` types into `$x`. `Slider` steps or clicks `$x`. `Globe` rotates `$x` as `yaw` |
| `selected: $i` | `Select`/`Tabs`/`Menu`/`WidgetList` arrows and clicks write the index. `Tree` writes the identifier path |
| `checked: $b` | `Checkbox`/`Switch` toggles `$b` |
| `offset: $n`, `position: $n` | Scroll zones. Wheel, arrows and track clicks write the offset |
| `choice: $s` | `Dialog` writes `"yes"` or `"no"` on commit |
| `open: $b` | `Dialog` draws while `$b` is true |
| `on click:` / `on select:` / `on change:` / `on choose:` | Fires the named `call:` handler |

## Core widgets

These map to `ratatui::widgets` or wrap them with Forge props.

| Element | Source | Key props |
| --- | --- | --- |
| `Block` | `ratatui::widgets::Block` | `title`, children or text |
| `Paper`, `Card` | `Block` aliases | same |
| `Gauge` | `ratatui::widgets::Gauge` | `value` percent, `label` |
| `Progress` | `Gauge` alias | same |
| `LineGauge` | `ratatui::widgets::LineGauge` | `value`, `label`, `symbol` |
| `List` | `ratatui::widgets::List` | `items`, `selected` |
| `Table` | `ratatui::widgets::Table` | `columns`, `rows` |
| `Tabs` | `ratatui::widgets::Tabs` | `items`, `selected` |
| `Select`, `Menu` | `List` variants | `items`, `selected` |
| `Input`, `TextInput` | `Paragraph` + cursor | bound `value`, `placeholder` |
| `Button` | styled `Paragraph` | `on click:` |
| `Checkbox`, `Switch` | styled `Paragraph` | bound `checked` |
| `BarChart` | `ratatui::widgets::BarChart` | `data`, `labels`, `bar_width`, `max` |
| `Chart` | `ratatui::widgets::Chart` | `data`, `name`, axis bounds |
| `Sparkline` | `ratatui::widgets::Sparkline` | `data`, `max` |
| `Calendar` | `Monthly` (widget-calendar) | `date` as `YYYY-MM[-DD]` |
| `Canvas` | `ratatui::widgets::canvas` | `map` draws the world map |
| `Scrollbar` | `ratatui::widgets::Scrollbar` | `orientation`, `content_length`, bound `position` |
| `Logo` | `RatatuiLogo` | `size` tiny or small |
| `Mascot` | `RatatuiMascot` | — |

## Forge primitives

Layout and text helpers the core set lacks.

| Element | Behavior |
| --- | --- |
| `VStack`, `HStack` | Split the area along an axis by child constraints |
| `Spacer` | Empty area. Style `size` feeds the parent constraint |
| `Text` | Styled paragraph. `wrap` reflows, default on |
| `Title` | Bold text |
| `Divider` | One-row horizontal rule |
| `Log` | Scrollable text. Bound `offset` scrolls |
| `Fill` | Tiles `symbol` over the whole rect |
| `Clear` | Clears its rect for overlays |

## Extension widgets — text and content

| Element | Crate | Notes |
| --- | --- | --- |
| `Markdown` | [ratatui-markdown](https://crates.io/crates/ratatui-markdown) | Native-only. Bound `offset` scrolls. `width` sets wrap |
| `AnsiText` | [ansi-to-tui](https://crates.io/crates/ansi-to-tui) | ANSI escapes become styled spans |
| `Terminal` | ansi-to-tui + `TestBackend` | Paints an ANSI screen as a bordered cell grid. Bound `offset` reads scrollback |
| `CodeEditor` | [ratatui-code-editor](https://crates.io/crates/ratatui-code-editor) | Native-only. `language` picks the grammar. `original` shows a diff. `folding` and bound `offset` |
| `BigText` | [tui-big-text](https://crates.io/crates/tui-big-text) | Native-only. 8x8 pixel font. `size` picks the pixel density. `align` |
| `Image` | [image](https://crates.io/crates/image) | Decodes `src` (resolved on `base_dir`) into `▀` halfblock cells |
| `QrCode` | [qrcode](https://crates.io/crates/qrcode) | Unicode halfblocks from `data` |
| `Shimmer` | [tui-shimmer](https://crates.io/crates/tui-shimmer) | Sweep highlight. `phase` 0..1 pins a frame. Bind it to animate |
| `Globe` | [tui-globe](https://crates.io/crates/tui-globe) | Native-only. Embedded coastline mesh. `yaw`/`pitch`/`zoom` — a bound `yaw` rotates on arrows |

## Extension widgets — lists and data

| Element | Crate | Notes |
| --- | --- | --- |
| `Tree` | [tui-tree-widget](https://crates.io/crates/tui-tree-widget) | `items` indented text or `TreeItem` children. Bound `selected` is the identifier path. Arrows navigate and fold |
| `TreeItem` | child of `Tree` | `id` overrides the row text as identifier |
| `WidgetList` | [tui-widget-list](https://crates.io/crates/tui-widget-list) | Children are items of mixed height. Bound `selected`. `direction: horizontal` scrolls sideways |
| `Nodes` | [tui-nodes](https://crates.io/crates/tui-nodes) | `GraphNode` children auto-place. `edges: "0.0>1.0"` wires ports. `lines` picks the edge style |
| `GraphNode` | child of `Nodes` | One box. `width`/`height` in cells |
| `PieChart` | [tui-piechart](https://crates.io/crates/tui-piechart) | Native-only. `data`/`labels`. `hole` 0..0.9 makes a donut. `legend`/`legend-position` |

## Extension widgets — overlays and input

| Element | Crate | Notes |
| --- | --- | --- |
| `Popup` | [tui-popup](https://crates.io/crates/tui-popup) | Children float centered in a bordered surface. `width`/`height` size the body |
| `Toast` | [ratatui-toaster](https://crates.io/crates/ratatui-toaster) | Bordered card. `type` tints (info/success/warning/error) |
| `Dialog` | [tui_confirm_dialog](https://crates.io/crates/tui_confirm_dialog) | `open` shows the centered box. `choice` binds the answer. `yes-label`/`no-label` take `"(Y)es"` markup |
| `Slider` | [tui-slider](https://crates.io/crates/tui-slider) | Native-only. Bound `value`. `min`/`max`/`step`. `orientation`, `label`, `show-value` |
| `ScrollView` | [tui-scrollview](https://crates.io/crates/tui-scrollview) | Children render into a virtual `width`x`height` buffer. Bound `offset` scrolls. `scrollbars` |
| `Fx` | [tachyonfx](https://crates.io/crates/tachyonfx) | Post-processes children cells. `kind` picks the effect. `progress` 0..1 scrubs `duration` ms |

### `Fx` effect kinds

`dissolve`, `coalesce`, `dissolve-to`, `coalesce-from`, `sweep-in`,
`sweep-out`, `slide-in`, `slide-out`, `explode`, `expand`, `expand-v`,
`stretch`, `paint`, `fade-to`, `hsl-shift`. A fresh `Effect` builds each
render, so `progress` maps deterministically to a frame of the effect.

## Native-only widgets

Four widgets pull `crossterm` as a required dep, which has no wasm32
build. `CodeEditor` compiles tree-sitter C grammars and `Markdown`
enables ratatui's crossterm backend. On the WASM canvas these arms fall
through to the placeholder text.

| Element | Reason |
| --- | --- |
| `Slider`, `PieChart`, `BigText`, `Globe` | required `crossterm` dep |
| `CodeEditor` | tree-sitter C sources |
| `Markdown` | ratatui's default `crossterm` feature |

## Skipped crates

Reviewed during the awesome-ratatui sweep and deliberately not wrapped.

| Crate | Reason |
| --- | --- |
| `tui-file-explorer`, `scm-record` | App-level components with their own event loops, not doc widgets |
| `boxmaid` | Targets ratatui 0.29 |
| `term-maths` | Pulls a LaTeX parser for a niche surface |
| `ratatui-image`, `ratatui-splash-screen` | Sixel/kitty stacks are dead code on a cell grid. Native `Image` covers it |
| `ratzilla` | A different web story than `forge-web-canvas` |
