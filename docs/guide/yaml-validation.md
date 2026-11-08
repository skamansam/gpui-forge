# Validating `.forge.yaml` documents

Forge validates documents two ways — live inside Forge Studio's editor,
and in any YAML-capable editor via the JSON Schema.

## In Forge Studio

Studio runs `forge check` on every edit:

- **Diagnostics panel** — below the editor, `Diagnostics` shows every
  problem `check::check` finds (unknown elements, missing props,
  `forge.*` intrinsic errors, base-namespace mismatches). It updates
  live as you type.
- **Problem count** — `studio.problems` mirrors the count in the
  inspector/status area.

No setup — it's built in.

## In other editors — the JSON Schemas

Forge ships a JSON Schema **per base**, generated from the catalog manifests —
element names, per-element props, events, content types, `use:` libraries and
`style:` keys are all validated. They're published with the docs site under
`schema/0.1/`:

```text
https://gpui-forge.rudeboy.dev/schema/0.1/gpui-kit.json
https://gpui-forge.rudeboy.dev/schema/0.1/gpui02.json
https://gpui-forge.rudeboy.dev/schema/0.1/adabraka.json
https://gpui-forge.rudeboy.dev/schema/0.1/bezel.json
https://gpui-forge.rudeboy.dev/schema/0.1/ratatui.json
https://gpui-forge.rudeboy.dev/schema/0.1/iced.json
https://gpui-forge.rudeboy.dev/schema/0.1/generic.json   ← every catalog, any base
```

`generic` accepts all registered elements — pick it when a document declares
no `base:` or when mapping a glob for an editor.

### Per-document `$schema` field — the shipped docs use this

Forge documents accept an optional top-level `$schema` field that
yaml-language-server (VS Code, Zed, Neovim) picks up without any editor
config:

```yaml
$schema: https://gpui-forge.rudeboy.dev/schema/0.1/gpui-kit.json
forge: "0.1"
base: gpui-kit
root: ...
```

### Modeline alternative

```yaml
# yaml-language-server: $schema=https://gpui-forge.rudeboy.dev/schema/0.1/gpui-kit.json
forge: "0.1"
...
```

### VS Code (redhat.vscode-yaml) — workspace glob

In `settings.json`:

```json
"yaml.schemas": {
  "https://gpui-forge.rudeboy.dev/schema/0.1/gpui-kit.json": "**/*.forge.yaml"
}
```

### JetBrains IDEs

`Settings → Languages & Frameworks → Schemas and DTDs → JSON Schema Mappings` —
add the schema URL + the `*.forge.yaml` mapping. JetBrains also honors the
`# $schema: <url>` first-line comment.

### Regenerating the schemas

The schemas are generated from `catalogs/*.catalog.yaml`,
`bases/*.base.yaml` and `style::KEYS` — run `cargo xtask schemas` after
editing any of them (CI fails on drift). `forge schema --yaml [--base <id>]`
prints one to stdout; plain `forge schema` still emits the schema for the
document *model* (the `.forge.json` form).

## What the schema covers

- **Structure** — `forge`/`$schema`/`base`/`use`/`meta`/`state`/`root`/`Dialog#name:`.
- **Elements** — base elements unqualified (`Button#save`), libraries via the
  import alias (`ely.Button`); unknown names are flagged.
- **Props & events** — per-element names (`Button.variant`), value types,
  enum values, `required` props, `children` restrictions, `on click`-style
  handler keys.
- **`use:`** — alias shape plus the library ids registered for the base.
- **`style:`** — all `style::KEYS` entries with their value shapes.
- **Hover docs** — catalog `summary` fields surface as property descriptions.

`forge-spec` validation stays authoritative: unique ids, binding rules
(`bind` on non-bindable props, two-way inside lists), required-prop
satisfaction via positional content, and base compatibility are runtime
checks the schema only approximates.
