# Projects and files

A Forge project is basically one YAML file. Studio's file model follows
that — you can keep a document in memory for as long as you like. A folder
is only needed once you do something that requires one.

## The File menu

| Item | What it does |
|---|---|
| **New** | An empty gpui-kit document, unsaved. |
| **New Project…** | A scaffolded document — also unsaved. No folder prompt: a path is only asked for when you save or build. |
| **Open…** | A `.forge.yaml` document. |
| **Open Project…** | A `forge.toml` — its [`forge.document`](forge-toml.md) resolves the doc, and the manifest dir becomes the project dir for `src:` and `base_dir` resolution. |
| **Preview in Host…** | Spawn the [native preview host](studio-preview.md) for the document's base. |
| **Format** | Canonical formatting of the current source. |
| **Save** | Writes in place. On an unsaved document it becomes Save As and asks for a location. |
| **Preferences…** | The [preferences dialog](studio-themes.md). |

Recent files live in the menu too — the list persists at
`~/.local/share/gpui-forge/recent.txt`.

## The project sidebar

The left rail is a file viewer rooted at the open document's folder — the
last-opened project root, or Studio's launch directory before anything
loads. It lists folders and `*.forge.yaml` files. It filters out VCS and
build noise (`target`, `node_modules`, dot-files), and caps the scan's
depth and entry count so a huge tree can't stall the UI. The panel is
resizable. Clicking a file opens it in the **Document** tab.

Right-clicking a row offers file actions: **New document here** creates the
first free `untitled{,-N}.forge.yaml` in that folder, seeded with the open
document's base, and opens it. **Reveal in file manager** opens the OS file
browser at the row's folder. **Rename…** prompts for a new name —
`*.forge.yaml` files keep their suffix. Renaming the open document re-points
saves at the new path.

## Saving

Save writes the *preserving* writer's output — your anchors, comments, and
formatting stay put wherever the writer can prove they're unchanged. The
status bar shows the open `file` path and `project` dir.

## The Project tab

With a `forge.toml` open, a **Project** tab appears next to the document
tabs (`if: $studio.project`). It edits the manifest itself — the file
loads into an editor, and **Apply** writes `forge.toml` back to disk and
re-loads the project, so `document`/`icons`/`assets`/`themes` changes take
effect without reopening.

## Watching for external changes

Watching is declared, not automatic. `meta: {watch: "true"}` on a document
(or `[forge] watch = true` in the project manifest, which also watches
`forge.toml` itself) tells Studio to poll the file: a change re-checks and
re-renders it — no manual refresh — and a broken edit keeps the last-good
copy while the parse error lands in the status bar and diagnostics markers.
Studio's own shell (`studio.forge.yaml`) opts in the same way.

## Dialogs

`Dialog#name:` declarations open as real windows via `open_dialog("name")` —
Studio's own New Project and Preferences dialogs are Forge dialogs, with their
own bound state. Your documents get the same treatment: a doc that declares a
dialog can open it from a menu or button.

## From document to app

When a document is ready to ship, the CLI takes over:
[`forge build`](forge-cli.md) scaffolds a native app crate around it and
[`forge package`](forge-cli.md) produces an OS bundle. See the
[forge.toml reference](forge-toml.md) for what a project manifest adds.
