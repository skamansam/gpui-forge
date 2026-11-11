# `forge.toml` — the project manifest

A document is one file. A *project* wraps it in a `forge.toml`. The manifest
tells the CLI which document is the entry point, which base it targets, and
how to package the app. Studio's **Open Project…** reads the same file —
its `forge.document` resolves the document and sets `dir` so relative asset
paths resolve.

`forge new <name>` writes this scaffold:

```toml
[package]
name = "my-app"
version = "0.1.0"
id = "dev.forge.my-app"

[forge]
base = "gpui-kit"
document = "app.forge.yaml"

# [assets]
# dir = "assets"

# [libraries]
# ely-gpui-components = "^0.4"
```

## `[package]` — app metadata

| Key | Meaning |
|---|---|
| `name` | App name — also the default output binary name for `forge build`. |
| `version` | App version, carried into bundles. |
| `id` | Bundle/application id, e.g. `dev.example.my-app`. |
| `icon` | Icon path for packaged apps. |
| `title` | Window title for built apps. |
| `binary` | Optional: name of an *existing workspace crate* (e.g. `forge-studio`) — `forge package` release-builds it directly instead of scaffolding a generated app. |

## `[forge]` — the document

| Key | Meaning |
|---|---|
| `base` | The [base component system](forge-yaml.md#base-and-library-prerequisites) the document renders on. Checked against the shipped registry. |
| `document` | Entry document, relative to the manifest — what bare `forge build`/`check`/`preview` resolve. |

## `[assets]` — static files

| Key | Meaning |
|---|---|
| `dir` | Directory of assets bundled with the app. `src:` props resolve relative to it. |

## `[hooks]` — Rust glue

| Key | Meaning |
|---|---|
| `krate` | Path to a Rust crate that will register the document's hook handlers — declared, but not yet wired into `forge build`. Today [`forge hooks <doc>`](forge-cli.md) emits the JSON manifest of `on <event>: name()` calls a hooks crate must implement. |

## `[libraries]` — `use:` dependencies

Version pins for external component libraries — map of library name →
version requirement. `forge build` turns each entry into a Cargo dependency
line in the scaffolded app and links the matching adapter crate for every
`use:` import the document makes.
