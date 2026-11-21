# State Inspector — `state:` + `forge.Vars`

Every bound variable in a Forge document is inspectable and editable live
through `forge.Vars`, a scrollable state inspector. It reads the *edited
document's* state (not the Studio app's own `self.state`).

## `state:` — declared variable types

```yaml
state:
  studio.theme: string
  studio.dark: bool
  studio.light_themes: list
  editor.source: multiline
  preview.mode: enum(remote | embedded)
  slider: number
```

`state:` entries declare each variable's type — `forge.Vars` uses the type
to pick the right editor widget. `forge check`/`forge build` handle them like
`meta:`/`use:`.

When a document loads into a `ForgeView` (`set_document`), every declared
path seeds to its type's zero value — `string`→`""`, `bool`→`false`,
`number`/`slider`→`0`, `list`→`[]`, `object`/`map`→`{}`, `color`→`"#000000"`,
`enum(a | b)`→first variant — *only* where the path is absent or null, so
live values survive a reload. This is the same initialization
`forge build --codegen` emits for a built app, and it's why the Vars table
lists declared vars before anything writes them.

## `forge.Vars` — the inspector

```yaml
- forge.Vars:
    of: canvas                     # the `forge.Preview` id whose document state is inspected
    count: bind $studio.vars_count # optional — publishes the flattened row count
```

- **`of`** names the `forge.Preview` element id (default `canvas`) — writes go
  into the *edited document's* nested ForgeView, so `{$path}` bindings in the
  preview tab react to edits live.
- **`count:`** — bound, publishes the number of rows the table shows (every
  leaf path in the state JSON, sorted). A doc reads it for a "N variables"
  line instead of re-flattening state.
- **Table layout** — fixed label column + aligned editor column, paths
  ellipsized with the full name on hover.
- **Scalars** — `string`/`number`/`bool` get a subscribed `Input` that writes
  `update_state` on `InputEvent::Change`.
- **Objects/arrays** — a `Textarea` pre-filled with pretty JSON. `set` parses
  it back to `update_state`.
- **`bool`** → `Checkbox`, **`number`** → `NumberInput`, **`slider`** →
  `Slider`, **`enum(a | b | c)`** → `Select` (variants parsed from the type
  string), **`color`** → `Input` (hex). Every typed editor writes its change
  back through `update_state`.
- **"add" row** — two `Input`s write `vars_add.{path,value}`. "+" commits
  `update_state(&path, parsed_value)` — new vars appear instantly.

## `forge build --codegen` — `set_state` boilerplate

`forge build --codegen` emits `pub fn set_state(view, cx)` alongside `pub fn
document()`. It walks the doc's `bind`/`$path`/`{$path}`/`update_state`/
`set_path` references to collect every state path. It then emits
`view.set_state(json!({...}))` with all paths initialized to their declared
types (`""`/`0`/`false`/`[]`/`"first-variant"`/`"#000000"`) so the generated
app knows which vars it must wire before `set_document`.
