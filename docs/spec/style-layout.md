# Style & layout model

Status: **draft** (spec version `0.1`). Tracks [#5](https://github.com/skamansam/gpui-forge/issues/5).

A node's `style:` map holds presentation that is not a component prop: flex layout, sizing,
spacing, borders, colors and text. Every key maps onto one family of GPUI `Styled` methods, and
values use the names of those methods (and of gpui-kit's theme fields), so a document reads like
the Rust it becomes. Component-specific presentation (button variants, component sizes) stays in
props, defined by each library's catalog.

```yaml
- HStack:
  style:
    gap: 8px
    padding: [8px, 12px]   # y x, like CSS
    align: center          # items_center()
    justify: between       # justify_between()
    background: $theme.muted
    radius: md
```

## Values

| Type | Written as | Notes |
|---|---|---|
| length | `12px`, `1.5rem`, `50%`, `0` | bare numbers are pixels; `%` is relative to the parent |
| size | a length, `auto` or `full` | `full` is `w_full()` / `h_full()` |
| edges | one value, `[y, x]` or `[top, right, bottom, left]` | for `padding`, `margin` |
| color | `"#rrggbb"`, `"#rrggbbaa"` or `$theme.<field>` | colors must be quoted in YAML |
| keyword | one of the listed words | lower case, the suffix of the GPUI method |

`$theme.<field>` names a field of gpui-kit's `ThemeColor` in its Rust spelling, e.g.
`$theme.primary`, `$theme.muted_foreground`, `$theme.border`, `$theme.danger`. Theme tokens keep a
document correct in light and dark themes; prefer them to literal colors.

## Keys

### Flex container

| Key | Values | GPUI |
|---|---|---|
| `direction` | `row`, `column`, `row-reverse`, `column-reverse` | `flex_row()`, `flex_col()`, … (`VStack`/`HStack` set it already) |
| `gap`, `gap-x`, `gap-y` | length | `gap()`, `gap_x()`, `gap_y()` |
| `align` | `start`, `center`, `end`, `stretch`, `baseline` | `items_*()` (cross axis) |
| `justify` | `start`, `center`, `end`, `between`, `around`, `evenly` | `justify_*()` (main axis) |
| `wrap` | `true` / `false` | `flex_wrap()` / `flex_nowrap()` |

### Flex item

| Key | Values | GPUI |
|---|---|---|
| `grow` | number, or `true` (= 1) | `flex_grow()` |
| `shrink` | number, or `false` (= 0) | `flex_shrink()` |
| `basis` | size | `flex_basis()` |
| `align-self` | `start`, `center`, `end`, `stretch`, `baseline` | `self_*()` |

### Size and spacing

| Key | Values | GPUI |
|---|---|---|
| `width`, `height` | size | `w()`, `h()`, `w_full()`, … |
| `min-width`, `max-width`, `min-height`, `max-height` | size | `min_w()`, `max_w()`, … |
| `padding` | edges | `p()`, `px()`/`py()`, `pt()`/`pr()`/`pb()`/`pl()` |
| `margin` | edges | `m()`, … |
| `overflow` | `visible`, `hidden`, `scroll` | `overflow_hidden()`, `overflow_scroll()` |

### Border and background

| Key | Values | GPUI |
|---|---|---|
| `border` | length (width) | `border()` |
| `border-color` | color | `border_color()` |
| `border-style` | `solid`, `dashed` | `border_dashed()` |
| `radius` | length, or `sm`, `md`, `lg`, `full` | `rounded()`, `rounded_full()`; `sm`/`md`/`lg` follow gpui-kit's theme radius |
| `background` | color | `bg()` |
| `opacity` | number `0`–`1` | `opacity()` |
| `shadow` | `none`, `sm`, `md`, `lg`, `xl` | `shadow_*()` |

### Text

Text keys apply to the element and are inherited by its children, as in GPUI.

| Key | Values | GPUI |
|---|---|---|
| `color` | color | `text_color()` |
| `size` | `xs`, `sm`, `base`, `lg`, `xl`, `2xl`, `3xl`, or a length | `text_xs()` … `text_3xl()`, `text_size()` |
| `weight` | `thin`, `light`, `normal`, `medium`, `semibold`, `bold`, `black` | `font_weight(FontWeight::…)` |
| `text-align` | `left`, `center`, `right` | `text_left()`, … |
| `line-height` | length | `line_height()` |
| `italic`, `underline`, `truncate` | `true` / `false` | `italic()`, `underline()`, `truncate()` |

## Validation

`forge_spec::validate` checks every `style:` entry: unknown keys are errors (with the closest key
as a hint), and values must have the key's type. Bindings are allowed in values (`$ui.width`) and
checked at runtime; two-way bindings (`bind …`) are not allowed in `style`.

## Not in 0.1

Grid layout, absolute positioning, transitions and responsive variants. They will be added as new
keys, so existing documents stay valid.
