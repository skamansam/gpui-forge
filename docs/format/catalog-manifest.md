# Catalog manifests

A **catalog manifest** describes the elements a component library offers to Forge: their props
and prop types, events, allowed children, and defaults. Validation, the Studio and forge-web
palettes and inspectors, `forge schema`, and the CLI all read catalogs from manifests; nothing
about a library's elements is hard-coded.

Each library adapter ships one manifest. The gpui-kit base library's manifest is
[`catalogs/gpui-kit.catalog.yaml`](../../catalogs/gpui-kit.catalog.yaml); it moves into the gpui-kit
adapter crate once that exists (#75).

Manifests are YAML, read with the same parser as Forge documents, and validated by editors
through [`schema/catalog-0.1.json`](../../schema/catalog-0.1.json). The loader is
`forge_spec::catalog::from_yaml`.

## Example

```yaml
catalog: "0.1"            # manifest format version
library: gpui-kit         # library id; documents import it by this id (`use:`)
name: GPUI Kit
version: 0.8.0            # library version the manifest describes
license: Apache-2.0
bases:                    # bases this library works on, with a version requirement
  gpui-kit: "^0.8"

elements:
  Button:
    summary: Clickable button.
    component: gpui_component::button::Button   # implementing type, for docs and adapter authors
    content: label        # `Button: Sign in` sets `label`
    props:
      label: string       # shorthand for `{ type: string }`
      variant:
        type: enum
        values: [Default, Primary, Danger]   # the library's Rust variant names
        default: Default
      disabled:
        type: bool
        default: false
    events:
      click:              # no payload
  Input:
    content: value
    props:
      value:
        type: string
        bindable: true    # accepts `bind $form.email`
    events:
      change: string      # shorthand for `{ payload: string }`
  Tabs:
    children: [Tab]       # `any`, `none` (default) or a list of element names
```

## Elements

| Key | Meaning |
|---|---|
| `summary` | One-line description shown in palettes and editor hovers. |
| `component` | The library type that implements the element. Documentation only. |
| `content` | Prop that receives the node's positional content. Must be one of `props`. |
| `props` | Prop name → type shorthand or prop definition. |
| `events` | Event name → empty, payload type shorthand, or `{ payload, summary }`. Documents bind them with `on <event>:`. |
| `children` | `none` (default), `any`, or a list of allowed element names. A bare name is an element of the same library; `<library-id>.Element` names another library's element (import aliases are chosen per document, so catalogs use library ids). |

Element names are PascalCase. Props and events are lower-case identifiers. Styling is not part of
the catalog: `style` keys come from the style and layout model (#5) and apply to every element.

## Props

| Key | Meaning |
|---|---|
| `type` | `string`, `number`, `length`, `bool`, `color`, `icon`, `enum`, `list<T>`, or `map`. |
| `values` | Allowed bare words, for `enum` (and `list<enum>`). |
| `default` | Default value, written in the Forge value syntax and checked against `type`. |
| `required` | The prop must be set (directly or through `content`). Default `false`. |
| `bindable` | The prop accepts a two-way binding. Default `false`. |
| `summary` | Description for the inspector. |

What each type accepts from a document (see [Forge YAML values](forge-yaml.md#values)):

| Type | Accepts |
|---|---|
| `string` | text, interpolated text, bare words, numbers, booleans |
| `number` | unitless numbers |
| `length` | numbers with or without a unit |
| `bool` | `true` / `false` |
| `color` | quoted colors |
| `icon` | an icon name |
| `enum` | one of `values` |
| `list<T>` | a YAML list whose items are all `T` |
| `map` | a YAML mapping; nested keys are the elements own contract |

One-way bindings (`$path`) are accepted by every type and checked at runtime. Theme tokens are
accepted by `color`, `length` and `number`.

## Bases

`bases` lists the base component systems the library works on. A project can only import
libraries whose manifest lists its base; the loader exposes `Catalog::supports_base` for that
check (#92). Version requirements are recorded but not evaluated yet.

## Adding a component

1. Add the element under `elements:` in the library's manifest, with its props, events and
   allowed children. Use the library's own names, including its Rust enum variant names for enum
   values (`Primary` for `ButtonVariant::Primary`, `Small` for `Size::Small`), so a document maps
   one-to-one onto the Rust API.
2. Map the element to the library's component in the library's runtime adapter (#74, #75).
3. Run `cargo test -p forge-spec`; the catalog tests load every manifest under `catalogs/`.
