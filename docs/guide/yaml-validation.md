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

## In other editors — the JSON Schema

`site/static/schema/forge-doc.schema.json` is the document schema
(also in the build output at `site/build/schema/`). Point your editor's
YAML language server at it.

### VS Code (redhat.vscode-yaml)

In `settings.json`:

```json
"yaml.schemas": {
  "https://raw.githubusercontent.com/skamansam/gpui-forge/main/site/static/schema/forge-doc.schema.json":
    "**/*.forge.yaml"
}
```

Or via a modeline at the top of the document:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/skamansam/gpui-forge/main/site/static/schema/forge-doc.schema.json
forge: "0.1"
...
```

### JetBrains IDEs

`Settings → Languages & Frameworks → Schemas and DTDs → JSON Schema Mappings` —
add the schema URL + the `*.forge.yaml` mapping.

### Per-document `$schema` field

Forge documents accept an optional top-level `schema` field — editors
that auto-detect it (yaml-language-server) pick the schema up without
editor config:

```yaml
schema: "https://raw.githubusercontent.com/skamansam/gpui-forge/main/site/static/schema/forge-doc.schema.json"
forge: "0.1"
base: gpui-kit
root: ...
```

## What the schema covers

The schema validates the document *shape* — `forge`/`base`/`root`/`use`/
`props`/`bindings`/`styles`/`events`/`dialogs` structure. Element-level
validation ("is `ely.Button` a real element for this base") needs the
base's catalog — `forge check <doc>` (or Studio's Diagnostics panel)
does that check against the live catalogs; the schema alone can't know
which base+libraries the doc uses.
