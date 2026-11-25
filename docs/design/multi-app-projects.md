# Projects and multi-file builds — design outline

Status: **draft**. Extends [project-manifest.md](project-manifest.md), which
already settled that the manifest is optional. This note removes the last
implicit behavior — cwd-based discovery — and makes a project a grouping of
applications, not of files.

## Principles

1. **The document is definitive.** A `.forge.yaml` file is the source of
   truth for its own build. What it declares, stands.
2. **A project is a convenience.** It exists to make managing multiple
   files, assets and applications easier. Where the document does not
   specify, the project fills the gap. A project can never contradict a
   document.
3. **Defaults cover the rest.** When neither the document nor the project
   declares a value, a sane built-in default applies — a missing field is
   never an error on its own. Defaults may produce surprising results for an
   underspecified file, but they are predictable and documented.
4. **A project is explicit.** Tools open a project only when the user names
   one: a `forge.toml` path on the command line, or **Open Project…** in
   Studio. There is no walk-up. `forge build app.forge.yaml` builds
   `app.forge.yaml` — whatever `forge.toml` happens to sit above cwd is
   irrelevant.
5. **A project groups applications, not files.** An application is one entry
   document plus the documents that belong to it. One project can hold
   several applications, and the applications can target different bases.
6. **Membership is declared in the document.** A file says which application
   it belongs to. The manifest may list applications for convenience, but a
   file dropped next to the project still works without a manifest edit.

## What changes

### No implicit manifest discovery

`load_manifest` today walks up from cwd looking for `forge.toml`. Under this
outline it only runs when the user passes a project:

| Invocation | Context |
|---|---|
| `forge <cmd> app.forge.yaml` | Document context. `meta:` supplies project data. |
| `forge <cmd> ./dir/` or `forge <cmd> dir/forge.toml` | Project context. The manifest applies. |
| `forge <cmd>` (bare) | Looks for `forge.toml` in cwd only — no parent walk. |

Studio already matches this: **Open File** is document context, **Open
Project…** is project context. The change removes `project()`'s upward
search, so a stray `forge.toml` above the working directory can no longer
rebrand a build.

### Document-level project data

`meta:` gains the fields that today only exist on the manifest:

| `meta:` key | Mirrors | Meaning | Default when absent |
|---|---|---|---|
| `meta.assets` | `[assets] dir` | Asset directory for `src:` props, relative to the document. | `assets/` beside the document, when it exists |
| `meta.include` | `forge.documents` | Other documents the application's build needs — dialogs, secondary windows, shared fragments. Paths relative to the document. | none — the document alone |
| `meta.window` | `forge.document` | The application's entry document — declared on *member* files, not the entry. See below. | none — the document is its own base window |

The precedence chain holds for every field, not only these. The document's
own declaration wins, the project value fills the gap, a sane default covers
the rest. `meta.title` → `[package] title` → file stem is one instance of
that chain. `base:` → `[forge] base` → `gpui-kit` is another.

### Applications and the base window

An application is a set of documents with one **entry document** — the *base
window*, the document that owns the app's window and `meta:` identity.

- **The entry document** is a base window: it declares the app through
  `meta:` (title, id, icon, assets, includes).
- **A member document** points at its application with
  `meta.window: <entry-doc>` — a relative path to the base window. A member
  without `meta.window` is a standalone document (or a project file not in
  any application).
- **One application, one base.** The base window's `base:` sets the
  application's base. Members must match it — same rule as today's
  per-binary base check, moved to per-application.
- **A project can mix bases across applications.** `admin.forge.yaml` on
  gpui-kit and `serial-console.forge.yaml` on ratatui can share a project —
  they produce separate app crates, not one binary.

The project manifest can name applications for discoverability:

```toml
# forge.toml — grouping only. Everything here has a doc-level equivalent.
[package]
name = "my-suite"

[[apps]]
document = "main.forge.yaml"     # base window for the default app

[[apps]]
document = "console.forge.yaml"  # a second app in the same project
```

`[forge] document` and `[forge] documents` keep working as the single-app
shorthand. `[[apps]]` is the multi-app form. `forge.base` becomes a default
for documents that don't declare `base:` rather than a hard requirement.

### The skeleton builds applications

`forge skeleton` currently emits one app crate for `forge.documents` on a
single base. Under this outline it emits **one crate per application**:

```
app/
  main/          # crate for main.forge.yaml's application
    Cargo.toml
    src/{main,docs,store}.rs
    docs/…
  console/       # crate for console.forge.yaml's application
    Cargo.toml
    src/{main,docs,store}.rs
    docs/…
```

Per application the emit is unchanged: the base window's `base:` selects the
`base-*` feature and the matching `main.rs` template, the doc set becomes
`docs::DOCS`, `use:` wires adapter crates. The doc set now comes from
`meta.include` + `meta.window` membership + `[[apps]]` — not from
`forge.documents` alone.

`forge build` and `forge package` gain `--app <name>` for project context.
Document context builds that document's application alone.

## Migration shape

1. `load_manifest` loses the upward walk — the first behavior change and the
   one that prompted this note.
2. `meta.assets` / `meta.include` / `meta.window` land in the document model
   and schema (`meta` is already a free-form map — this is validation +
   consumer wiring, not a format bump).
3. `skeleton_cmd` groups the doc set by application and loops the existing
   emit. Single-app projects are the degenerate case and keep today's output
   layout (`--dir <path>` → one crate).
4. The base-mismatch check moves from "document vs project" to "member vs
   its application's base window".

## Open questions

- Does `meta.window` need a name (`meta.window: console`), or is the entry
  doc path enough? A path is simpler. A name survives renames.
- How do `assets.dir` and `themes.dir` compose when both the entry doc and
  the manifest declare them — union, or doc-wins?
- Should `[[apps]]` entries carry per-app `name`/`binary` overrides, or does
  the base window's `meta:` fully own app identity?
- What does Studio's project view show for a multi-app project — one canvas
  per app, or a doc list grouped by application?
