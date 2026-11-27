# `forge.toml` — the project manifest

A document is one file. A *project* wraps it in a `forge.toml` — a
convenience for managing multiple files, assets and applications. The
document is definitive: what it declares stands. The manifest fills in what
the document leaves out, and a sane default covers the rest.

A project is only ever opened explicitly — a `forge.toml` path or project
directory on the command line, or **Open Project…** in Studio. `forge`
never climbs parent directories looking for one: `forge build app.forge.yaml`
builds that document alone, whatever manifest sits above the working
directory. A bare `forge <cmd>` checks the current directory only.

Studio's **Open Project…** reads the manifest — its `forge.document`
resolves the document and sets `dir` so relative asset paths resolve.

`forge new <name>` writes this scaffold:

```toml
[package]
name = "my-app"
version = "1.0.0"
id = "dev.forge.my-app"

[forge]
base = "gpui-kit"
document = "app.forge.yaml"

# [assets]
# dir = "assets"

# [libraries]
# ely-core = "^0.2"
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
| `base` | Default [base](forge-yaml.md#base-and-library-prerequisites) for documents that don't declare their own `base:`. A document's `base:` always wins. |
| `document` | Entry document, relative to the manifest — what bare `forge build`/`check`/`preview` resolve. |
| `documents` | Extra documents for `forge skeleton`, relative to the manifest — the single-app form of `[[apps]]`-era grouping. Documents with no `meta.window` land on the entry document's application. |
| `watch` | `true` opts the project into hot reload in Studio. Documents opened under it re-render on save (even without their own `meta.watch`), and the manifest itself is polled: edits to `document`/`icons`/`assets`/`themes` re-apply live. |

## `[[apps]]` — multiple applications

One project can hold several applications — each entry names the *base
window*, the entry document that owns the app's `meta:` identity. `forge
skeleton` emits one crate per application, so applications in a project can
target different bases.

```toml
[[apps]]
name = "main"                    # optional — defaults to the doc's file stem
document = "main.forge.yaml"

[[apps]]
document = "console.forge.yaml"  # e.g. a ratatui app beside a gpui-kit one
```

Documents join an application three ways:

- `meta.window: <entry doc>` on a member file points back at its base
  window — relative to the member's own directory.
- `meta.include: "<paths>"` on a base window pulls sibling documents in —
  dialogs, secondary windows, shared fragments.
- `forge.documents` entries with no `meta.window` land on the entry
  document's application.

`forge skeleton --app <name>` emits one application. `forge build --app
<name>` and `forge package --app <name>` select an application's base
window. `forge.document` stays the default application when `[[apps]]` is
absent.

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
