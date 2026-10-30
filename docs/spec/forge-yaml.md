# Forge YAML

Status: **draft** (spec version `0.1`). Tracks [#2](https://github.com/skamansam/gpui-forge/issues/2).

Forge documents are **plain YAML 1.2** with a JSON Schema, so any YAML parser, YAML↔JSON converter,
linter or schema-aware editor works on them. The same tree encoded as JSON is the Forge JSON format
([#6](https://github.com/skamansam/gpui-forge/issues/6)). Both encode the
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
| `forge` | yes | Spec version, as a string (`"0.1"`) |
| `base` | no | Base component system (`gpui-kit`, `bezel`, `adabraka`, `gpui-0.2`, and planned non-GPUI bases `ratatui`, `iced`); defaults to the project's `forge.toml` |
| `use` | no | Map of import alias → `"<library-id> [<version-req>]"` |
| `meta` | no | Map of string → string (title, description, …) |
| `root` | yes | The root node |

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
| `style` | map of style & layout properties ([#5](https://github.com/skamansam/gpui-forge/issues/5)) |
| `children` | list of nodes (order is significant, duplicates allowed) |
| `on <event>` | handler call: `sign_in`, `sign_in()`, `go_to($wizard.id, 2)` |
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

- `forge schema --base <base>` generates a JSON Schema from the base's and imported libraries'
  catalog manifests, so editors validate and complete element-specific props, not just structure.
- A published, structure-only schema per spec version lives at
  `https://gpui-forge.rudeboy.dev/schema/<version>/<base>.json`.
- Editor association: the `$schema` key, the `yaml-language-server` comment, or a `*.forge.yaml`
  file-name mapping (to be registered with SchemaStore). Which of these each editor honours is
  verified in [#6](https://github.com/skamansam/gpui-forge/issues/6).
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
