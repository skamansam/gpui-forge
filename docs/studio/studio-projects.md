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

The left rail is a file viewer rooted at the open document's folder (the
last-opened project root, or Studio's launch directory before anything is
loaded). It lists folders and `*.forge.yaml` files — VCS and build noise
(`target`, `node_modules`, dot-files) is filtered out and the scan is depth-
and entry-capped so a huge tree can't stall the UI. The panel is resizable.
Clicking a file opens it in the **Document** tab.

Right-clicking a row offers file actions: **New document here** creates the
first free `untitled{,-N}.forge.yaml` in that folder (seeded with the open
document's base) and opens it, **Reveal in file manager** opens the OS file
browser at the row's folder, and **Rename…** prompts for a new name —
`*.forge.yaml` files keep their suffix. Renaming the open document re-points
saves at the new path.

## Saving

Save writes the *preserving* writer's output — your anchors, comments, and
formatting stay put wherever the writer can prove they're unchanged. The
status bar shows the open `file` path and `project` dir.

## Watching for external changes

Studio watches the open file on disk: if another editor or `forge fmt -w`
rewrites it, Studio reloads the source and re-checks the document — no
manual refresh.

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
