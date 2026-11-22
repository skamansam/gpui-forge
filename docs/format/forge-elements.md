# `forge.*` elements — runtime tooling widgets

`forge.*` elements are rendered by the Forge runtime itself, not by a
component set. They always resolve. No `use:` import, no `base:`
requirement. And they exist so tooling UIs (icon browsers, inspectors,
file trees, palettes) can be built **in Forge** instead of embedding
native chrome into the runtime.

Three usage styles:

- **Standalone** — drop the element in and get a working widget with
  built-in chrome (pickers, search boxes, labels).
- **Data mode** — bind the element's *out-props* and it publishes its
  derived state instead of (or in addition to) rendering chrome. The doc
  lays the UI out itself with `for:`/`if:` and ordinary elements. This is
  how Forge Studio's own sidebars work — the generated UI is kept minimal
  and variables drive everything.
- **Scoped children** — an element that takes `children:` renders the
  template itself, once per item it owns, with the item pushed as `$item`
  (the same scope mechanism `for:` uses). The runtime keeps the parts the
  document can't express — measured layout, virtualization, load
  machinery — and the document owns what goes inside each frame. No
  `children:` → the element's built-in default renders, so a template is
  always an override, never a requirement.

### The custom-widget contract

Those last two styles are how a `forge.*` element becomes a *custom
widget*: **in-props parameterize, out-props publish data, `children:`
templates render inside the element's scope.** A custom widget is a
runtime piece. It knows what the doc language can't say — virtualizing
1800 rows, probing an asset source, walking a file tree. It declares the
rest. Its data and its template points. As bound props
and scoped children. Studio's inspector is `forge.Inspector` + `for:`.
its icon browser is `forge.Icons` + bound outputs + a `children:` cell
template. `List`'s `for:` child is the same idea for any element.

---

## `forge.Icons` — icon browser

A searchable, virtualized grid over the app's icon catalog.
`icons/<name>.svg` is `lucide` (bundled). `icons/<set>/<name>.svg` is any
set the host's asset source layers in. A project ships a set by dropping
SVGs under `<assets>/icons/<set>/` and naming it in `forge.toml`'s
`[icons] set`. An IconifyJSON set can instead be browsed over the
network and materialized locally on click.

### Inputs (bound or static)

| Prop | Type | Purpose |
| --- | --- | --- |
| `set` | string | Iconset to browse. Bound: two-way picker state. Default: the view's iconset (`meta.iconset` → `[icons] set` → `lucide`). |
| `sets` | string (path) | Publishes the discovered set list — e.g. `["heroicons","lucide"]`. |
| `query` | string | Search text. Bound: two-way — an external `Input` can own it. |
| `category` | string | Category filter. A bound value is reset to `"[all]"` when it isn't a category of the set. |
| `theme` | list\<string\> | Theme/suffix filter — `["s:filled","p:baseline-"]` encodings. Empty/all = no filter. |
| `size` | string | `XSmall` / `Small` / `Medium` / `Large` — glyph size, cell stride and row height. Default `Medium`. |
| `selected` | string | The clicked icon's name. The matching cell highlights. |
| `search` | bool | Show the built-in filter box (standalone mode). |
| `bare` | bool | Grid only — the doc builds its own chrome from the outputs below. |

### Outputs (data mode)

Bind these to state paths and `forge.Icons` publishes the same values its
built-in chrome renders. With `bare: true` the chrome isn't built at all.

| Prop | Publishes |
| --- | --- |
| `categories` | `["[all]", …]` — category names for the set (incl. a computed `Uncategorized`). |
| `themes` | `[{key, title, checked}]` — one entry per theme option. `key` is the `s:`/`p:` encoding `theme:` consumes, `checked` reflects the current filter. |
| `meta` | `{set, title, license, spdx, line1, line2}` — name, license/SPDX, plus two preformatted info lines. |
| `status` | `{downloading, label, percent, error}` — live set-download state for progress + retry UI. |
| `attribution` | bool — the set's SPDX requires shipping its notice with the built app. |

### Events

- `select(name)` — an icon cell was clicked. Studio's copy-to-`assets/icons` is one line: `on select: icon_copy`.

### Scoped children — the cell template

`children:` on `forge.Icons` replaces what goes **inside** each grid cell.
The template renders once per icon with `$item` pushed as
`{name, set, selected, downloaded}`:

- `name` — the icon's name (`home`), `set` — its set (`lucide`,
  `heroicons`, an iconify set)
- `selected` — the bound `selected:` value equals this icon's `name`
- `downloaded` — already materialized under `assets/icons/` (always
  false for bundled sets)

The runtime keeps the cell frame. The pinned column width that aligns
the grid, the selected accent fill, the downloaded ring, hover, tooltip
and the click that writes `selected`/`select`. Everything visual inside
the box is the doc's. Scope vars resolve in props (`name: $item.name`),
interpolate in text (`"{$item.name}"`), and bake into event args
(`on click: my_hook($item.name)`).

```yaml
- forge.Icons:
    bare: true
    set: bind $studio.iconset
    size: bind $studio.icon_size
    selected: bind $studio.icon
    on select: icon_copy
    children:
      - Icon: $item.name
        set: $item.set
        size: $studio.icon_size
      - Text: "{$item.name}"
        if: $item.downloaded
        style: {size: xs, color: $theme.success}
        else:
          - Text: "{$item.name}"
            style: {size: xs, color: $theme.muted_foreground}
```

Leave `children:` off and the built-in glyph + name cell renders. This is
exactly what Studio's Icons tab does — its cell layout is editable Forge
yaml.

### `Icon` — lazy icon element

`Icon` resolves names through the same probe the grid uses: `set:` (or a
`set:name` prefix on `name:`) picks the set. Missing iconify names queue
the set's load and show a placeholder until bytes land. Bundled
files stay canonical on collision. `size:` takes a px length or
`XSmall`/`Small`/`Medium`/`Large` (12/14/16/24px).

### Example — doc-owned chrome (the Studio pattern)

```yaml
- HStack:
  children:
    - Select: {options: $studio.iconsets, value: bind $studio.iconset}
    - Select: {options: $studio.icon_categories, value: bind $studio.icon_category, if: $studio.icon_categories}
    - Checkbox: "{$th.title}"
      for: $studio.icon_themes
      each: th
      checked: $th.checked
      on change: toggle_icon_theme($th.key)   # host hook — toggles the encoded key in studio.icon_theme
    - Select: {options: [XSmall, Small, Medium, Large], value: bind $studio.icon_size}
- Input: {value: bind $studio.icons_query, placeholder: filter icons…}
- Text: "{$studio.icon_meta.line1}"
  if: $studio.icon_meta.line1
- Alert: {if: $studio.icon_attribution, variant: Error, banner: true,
          message: "This set's license requires shipping its notice."}
- forge.Icons:
    bare: true
    set: bind $studio.iconset
    query: bind $studio.icons_query
    category: bind $studio.icon_category
    theme: bind $studio.icon_theme
    size: bind $studio.icon_size
    selected: bind $studio.icon
    categories: bind $studio.icon_categories
    themes: bind $studio.icon_themes
    meta: bind $studio.icon_meta
    status: bind $studio.icon_status
    attribution: bind $studio.icon_attribution
    on select: icon_copy
```

The grid *frame* stays internal — measured-width virtualization and the
per-cell "is this SVG materialized yet" probing aren't expressible in the
document language. Everything around it — and now everything inside each
cell — is.

---

## `forge.Inspector` — property table (data mode by design)

The other half of the pattern. Standalone it renders a catalog-driven
`Form`. Binding `items`/`styles`/`header` turns it into a pure data
provider that renders nothing:

```yaml
- forge.Inspector:
    of: canvas                    # the forge.Preview element id
    selected: bind $document.selected
    items: bind $inspector.items
    styles: bind $inspector.styles
    header: bind $inspector.header
- FormField: "{$item.label}"
  for: $inspector.items
  each: item
  children:
    - forge.InspectorField: {item: $item}
```

`items` entries are `{kind, name, label, type, options, description,
required, value}` — a leading `kind: ref` refname row then one
`kind: prop` per catalog prop. `styles` entries are
`{kind: style, name, label, type, options, value}` covering every
registered style key. `forge.InspectorField` dispatches on `kind`/`type`
to render the right editor (`bool`→Switch, `enum`→Select,
`color`→ColorPicker, `number`→NumberInput, else Input) and fires
`prop_changed`/`refname_changed`/`style_changed`, which hosts route to
document edits.

## `forge.Outline` — widget tree

```yaml
- forge.Outline: {of: canvas, selected: bind $document.selected, on select: select_node}
```

Renders the hosted preview's node tree (index paths `0.2.1`). Writes the
bound `selected` on click and highlights the externally-selected row.

## `forge.Files` — file tree

`of:` renders a JSON listing `[{name, path, dir, children}]` from state.
`root:` scans a directory itself (`filter:`/`depth:`/`hidden:` bound the
scan, `nonce:` busts the cache). `search:` adds a filter box.
`folders: navigate` makes dir rows enter instead of expand.
`highlights:` styles rows by glob. File clicks write `selected` and fire
`select` with the path. Virtualized — give it a bounded slot
(`grow` + `min-height: 0`).

## `forge.Palette`, `forge.Showcase`, `forge.Vars`

- `Palette` — the widget-catalog drag source. `all:` widens its search
  scope across sets.
- `Showcase` — the widget gallery used by `start: gallery`.
- `Vars` — the state/variables table for a `forge.Preview`'s document
  (`of:`). Rows edit in place with typed widgets (`bool`→Checkbox,
  `number`→NumberInput, `enum`→Select, objects→JSON Textarea). `count:`
  binds out the row total. See
  [state-inspector.md](state-inspector.md).

---

## Writing your own custom widget

`forge.*` are runtime built-ins. A *new* custom element is a Rust
adapter: implement `ForgeLibrary` (a catalog + a render function),
`view.register_library(MyLib)`, and documents use it via `use:` imports.
See [adapters.md](adapters.md) for the contract and
`crates/forge-lib-ely` for a full example.

When a custom widget wants Studio-style chrome, follow the same rule the
built-ins do: **bind, don't build**. Take bound props for state the doc
should own, publish derived lists/objects through out-props. Keep
only the parts the document language can't express (virtualization,
native pickers, network probing) inside the widget.
