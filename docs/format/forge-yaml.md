# Forge YAML

Status: **draft** (spec version `0.1`).

Forge documents are **plain YAML 1.2** with a JSON Schema, so any YAML parser, YAML↔JSON converter,
linter or schema-aware editor works on them. The same tree encoded as JSON is the Forge JSON format.
Both encode the
[document model](document-model.md).

File extension: `.forge.yaml` (`.forge.json` for JSON).

## At a glance

```yaml
# yaml-language-server: $schema=https://gpui-forge.rudeboy.dev/schema/0.1/gpui-kit.json
forge: "0.1"
base: gpui-kit
use:
  ely: ely-gpui-components ^0.4
meta:
  title: Login

root:
  VStack#root:
  style:
    gap: 12px
    padding: 24px
    align: center
  children:
    - Text: "Welcome back, {$user.name}"
      style:
        size: lg
    - Input#email:
      value: bind $form.email
      placeholder: Email
    - Button: Sign in
      variant: Primary
      disabled: $form.invalid
      on click: sign_in()
    - ely.Badge: beta   # libraries are addressed by their import alias
```

## Top-level keys

| Key | Required | Meaning |
|---|---|---|
| `$schema` | no | Schema URL (also accepted as a `# yaml-language-server: $schema=…` comment) |
| `forge` | yes | Spec version, as a string (`"0.1"`) — see Versioning below |
| `base` | no | Base component system (`gpui-kit`, `bezel`, `adabraka`, `gpui-0.2`, and planned non-GPUI bases `ratatui`, `iced`); defaults to the project's `forge.toml` |
| `use` | no | Map of import alias → `"<library-id> [<version-req>]"` |
| `meta` | no | Map of string → string (title, description, …) |
| `root` | yes | The root node |

### Base and library prerequisites

A `base:` picks which renderer runs the document. Most bases are pure Rust
and need nothing extra; a few libraries and bases require system packages at
build or run time:

| Base / `use:` library | Requirement |
|---|---|
| `gpui-video-player` | GStreamer 1.14+ and its plugins — [install guide](https://github.com/GStreamer/gstreamer-rs#installation) (Debian: `libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev`; macOS: `brew install gstreamer`; Windows: the MSVC runtime+devel MSIs) |
| `iced` | A GPU the app can open a surface on — wgpu needs Vulkan/Metal/DX12 (the web canvas targets WebGL2) |
| `gpui-kit`, `bezel`, `adabraka`, `gpui02`, `ratatui` | No system deps — pure Rust |

Everything else (`ely-core`, `gpui-component`, `gpui-flow`, `base-gpui`,
`gpui-apple-hig`, `gpui-pdf`, `gpui-whiteboard`, `ratex-gpui`) is pure Rust
and builds anywhere the base does.

## Fragments, anchors and merge keys

Forge supports standard YAML anchors, aliases and merge keys, so documents can share
repeated structure through top-level fragment slots. Any unknown top-level key whose
value is a mapping or sequence is treated as a fragment slot — it is not rendered or
validated as part of the document; scalar fragments go inside a container:

```yaml
forge: "0.1"

appMenu: &appMenu              # fragment slot — only referenced via *appMenu
  icon: hamburger
  children:
    - MenuItem: New
      on click: new_document()

fragments:                     # scalar anchors live inside a container
  label: &label "Save"

root:
  VStack:
  children:
    - DropdownMenu#file-menu:
      <<: *appMenu             # merge key — splices icon + children in
    - Button: *label           # alias — copies the anchored scalar
```

- `&name` anchors attach to scalars, sequences or mappings; `*name` aliases resolve to
  a clone of the anchored value and may appear anywhere — including as a whole node.
- `<<` merges a mapping (or a list of mappings, earlier winning) into the current one.
  Explicit keys always override merged ones.
- Aliases must reference anchors defined earlier in the document — saphyr rejects
  unresolved aliases, so the canonical writer always emits `name: &name` blocks
  before `root:` and dialogs.
- Anchored top-level maps become **named refs** (`Document.refs`): a `name: &name`
  block parses as a head-less node body — an entry-bag of props, `style:`, events
  and `children:` — and every `<<: *name` node records `refname`. Ref names must be
  unique; a second `&name` is a `duplicate ref` diagnostic (YAML permits shadowing,
  but refs are addressed by name).
- **Edits land in the ref, not the site.** When a merge-derived facet changes —
  an Inspector edit on a `<<: *appMenu` node — `Document::sync_refs` pushes the
  new facet into `refs.appMenu` and every other `*appMenu` site, so the preserving
  writer splices the change into the `appMenu:` block and leaves `<<: *appMenu`
  lines alone. Assigning a node's `refname` to a fresh name creates the block
  (`sync_refs` fills it from the node's own facets); clearing it flattens the
  merged entries back into the node.
- `yaml::to_string_preserving` writes documents back with their original source
  wherever the model did not change — anchors, aliases, merge keys, comments and
  formatting all survive a save; `yaml::to_string` remains the canonical
  formatter and emits the ref blocks followed by `<<` merge keys (never flattened).
- YAML tags (`!tag`) remain unsupported.

## Nodes

A node is a YAML mapping with exactly one **element key** plus optional reserved keys.

### Element key

```text
[alias.]Element[#id]      e.g.  Button   ely.Badge   Input#email
[alias.]Element[ .id]     e.g.  Input .email
```

- `Element` is PascalCase. `alias.` selects an imported library; base-library elements have no
  prefix (the base's own alias, e.g. `kit.`, may be written explicitly).
- The id may be written `#id` (no space: YAML only treats `#` as a comment after whitespace) or
  ` .id` (space, dot). Both parse to the same document. The formatter writes `#id` unless the
  project sets `id-style = "dot"` in `forge.toml`.
- The element key's **value** is the node's content (mapped by the catalog to the element's
  primary prop), or empty/null when there is none: `Button: Sign in`, `VStack#root:`.

### Reserved keys

| Key | Value |
|---|---|
| `style` | map of style & layout properties |
| `children` | list of nodes (order is significant, duplicates allowed) |
| `on <event>` | handler call: `sign_in`, `sign_in()`, `go_to($wizard.id, 2)` |
| `if`, `for`, `with`, `match` | control-flow heads — see [Control flow](#control-flow) |
| `else`, `is`, `not`, `each`, `index`, `as`, `case <v>` | control-flow sub-keys |
| anything else | a prop of the element (lower-case identifier) |

Keys are classified by shape: the element key matches `^([a-z][\w-]*\.)?[A-Z]\w*(#[\w-]+| \.[\w-]+)?$`,
`on <event>` keys start with `on `, and everything else is a prop. Exactly one element key per node.

Canonical key order written by the formatter: element key, props, `style`, events, `children`.

### Universal props

`tooltip` may appear on **any** element — it is not declared per-element and the validator does
not flag it as unknown. The runtime attaches the text as a hover tooltip around the rendered
element.

Binding paths also accept `-` in each segment, so style props can bind state by their catalog
names — e.g. `document.edits.style.text-align`.

### Control flow

Four lower-case control keys — `if`, `for`, `with`, `match` — condition, repeat or scope
nodes. They are reserved words, never element names (elements are PascalCase) and never
props. As a node's only key they form a **control block**; alongside an element key they
become a **modifier** on that element (the element repeats or conditionally renders).

```yaml
forge: "0.1"
root:
  VStack:
  children:
    # if — children render when the condition holds; else when it fails.
    # `is:`/`not:` compare the resolved value for equality; `else → if` chains
    # give the elsif shape.
    - if: $studio.preview
      is: gpui
      children:
        - Text: "GPUI Preview"
      else:
        - if: $user.admin
          children:
            - Text: Admin tools
          else:
            - Text: "No preview"

    # match — switches on one binding with `case <value>:` arms.
    - match: $studio.preview
      case gpui:
        - Text: "GPUI Preview"
      case web:
        - Text: "Web Preview"
      else:
        - Text: "No preview"

    # for — repeats children per item; `each:` names the scoped var ($item),
    # `index:` optionally names the index, `key:` identifies each item by a
    # member path ($item.id → stable render state across reorders), `else:`
    # renders when the list is empty. A single (non-list) value repeats once.
    - for: $studio.renderer_names
      each: item
      key: item.id
      children:
        - Text: "{$item}"
      else:
        - Text: "No renderers"

    # with — scopes one value under `as:` (the single-value template case).
    - with: $studio.selected_tab
      as: tab
      children:
        - Text: "Selected: {$tab.name}"

    # modifiers — the element itself repeats or conditionally renders.
    - Tab: "{$tab.name}"
      for: $studio.tabs
      each: tab
    - Text: "Signed in"
      if: $user.signed_in
```

- The control key carries the operand: `if:` a condition (truthy, or compared by `is:`/`not:`),
  `for:` a list binding (also a plain `[a, b]` list), `with:`/`match:` a binding. Use one-way
  `$path` bindings — `bind` is a write target and has no meaning here.
- `each`/`index`/`as` names create **scope vars**: `$item`, `$tab`, `$i` bind the scoped
  value anywhere inside the block (innermost scope wins; they shadow state paths and are
  read-only — `bind $item` cannot write back to the list).
- `case` operands are value literals — `case gpui:`, `case 2:`, `case 'a b':` — compared by
  value equality against the resolved `match` source; a binding also works (`case $mode:`).
- `else:` is valid on `if`, `for` (empty list) and `match` (no arm matched).
- Only one control key per node — nest blocks for combined conditions (`for` + `if` inside).
- `match` uses `case`/`else` instead of `children`, and cannot modify an element.

### File paths in `src`

`Image`'s `src` resolves **relative to the document file**, not the process cwd — a `.forge.yaml`
in `examples/` writes `src: ../shared/logo.svg` to reach the workspace root. `Image` renders
raster formats (PNG, JPEG, WebP, GIF, BMP, TIFF, ICO) and multicolor SVGs at full color via
gpui's `img()`; URLs and asset keys pass through to the app's `HttpClient`/`AssetSource`. Theme
icons come from `Icon` (Lucide names, tinted by `text_color`), not `Image`.

`DropdownMenu`/`MenuItem` render a button that opens a popover menu — `MenuItem` children fire
`on click:` hooks.

## Values

Forge reads every scalar as **text** and types it with the micro-syntax below. YAML's own typing
is ignored, so `0.1`, `on`, `no` or `1e3` are never turned into floats or booleans behind your back.
How a scalar is quoted selects the rules:

- **plain** scalars are fully typed (table below);
- **double-quoted** scalars are text with `{$path}` interpolation, or a color;
- **single-quoted** scalars are always literal text (no interpolation, no typing).

| Syntax | Model value |
|---|---|
| `Sign in`, `"Sign in"`, `'true'` | String |
| `Primary`, `center`, `Email` | Enum: a bare word. The catalog decides whether it is an enum value or text, so `placeholder: Email` is fine |
| `"Hi {$user.name}"` | Interpolated string (`{{` for a literal brace) |
| `12`, `1.5`, `-4px`, `1.5rem`, `50%`, `1fr` | Number with optional unit |
| `true`, `false` | Bool |
| `"#3b82f6"`, `"#fff"`, `"#00000080"` | Color, **must be quoted** (an unquoted `#` starts a YAML comment) |
| `$form.email`, `$items[0].name` | One-way binding |
| `$theme.colors.primary` | Theme token (bindings rooted at `theme`) |
| `bind $form.email` | Two-way binding (props only) |
| YAML sequence `[8px, 16px]` or a block list | List |

Event handlers are `name`, `name()` or `name(arg, ...)`, where arguments use the same value syntax
(`go_to($wizard.id, 2, 'next step')`). The formatter writes `name()`.

The formatter writes canonical output: key order element key, props, `style`, events, `children`;
colors as `#rrggbb[aa]`; and strings that a generic YAML 1.1 tool could mis-type (`yes`, `on`,
`0.1`, ...) single-quoted. Comments are not preserved by the formatter yet.

YAML gotchas the validator reports with a fix hint:

- `color: #fff` parses as an empty value plus a comment → *"expected a color; quote it: `\"#fff\"`"*.
- A value starting with `{` is a YAML flow mapping → quote interpolated strings that start with `{`.
- Values cannot start with `@` or `` ` `` in YAML; Forge does not use them.

## Schema and editor support

- `forge schema` emits the JSON Schema for the document model — generated from
  the serde types via schemars, so it cannot drift from `forge-spec`.
- `forge schema --yaml [--base <id>]` emits the JSON Schema for the YAML
  *syntax* — generated from the catalog manifests, so elements, props, events
  and `use:` libraries validate in editors. `cargo xtask schemas` regenerates
  the set published at `site/static/schema/0.1/` (`<base>.json` per base plus
  `generic.json`).
- `forge convert` turns a `.forge.yaml` document into the JSON form (and back),
  and `site/static/schema/forge-doc.schema.json` ships the generated model schema
  alongside generated TypeScript declarations for the web editor.
- Editor association: the `$schema` key, the `yaml-language-server` comment, or a
  `*.forge.yaml` file-name mapping — see the [validation guide](yaml-validation.md).
- Schema validation is a convenience. `forge-spec` validation is authoritative (unique ids,
  binding rules, base compatibility).

## Errors

The reader keeps source positions and reports `file:line:col` with a hint, e.g.

```text
login.forge.yaml:14:7: node has two element keys (`Text`, `Button`); split them into separate list items
login.forge.yaml:9:9: duplicate id `email` (first defined at 6:9)
```

## Worked examples

### Settings form with a library import

```yaml
forge: "0.1"
use:
  ely: ely-gpui-components ^0.4

root:
  VStack .settings:
  style:
    gap: 16px
    padding: 24px
  children:
    - Label: Notifications
      style:
        weight: semibold
    - HStack:
      style:
        justify: between
      children:
        - Text: Email me about updates
        - Switch#email_updates:
          checked: bind $settings.email_updates
          on change: save_settings
    - Select#theme:
      options: [light, dark, system]
      value: bind $settings.theme
    - ely.Stepper#step:
      value: $wizard.step
      on change: go_to_step($wizard.id)
```

### Minimal file

```yaml
forge: "0.1"
root:
  Text: Hello, world
```

### The same node in JSON

```json
{ "Button": "Sign in", "variant": "Primary", "disabled": "$form.invalid", "on click": "sign_in()" }
```
