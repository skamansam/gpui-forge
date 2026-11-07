# State Inspector — `state:` + `forge.Vars`

Every bound variable in a Forge document is inspectable and editable live
through `forge.Vars` — a scrollable state inspector that reads the *edited
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
to pick the right editor widget; `forge check`/`forge build` handle them like
`meta:`/`use:`.

## `forge.Vars` — the inspector

```yaml
- forge.Vars:
    of: canvas   # the `forge.Preview` id whose document state is inspected
```

- **`of`** names the `forge.Preview` element id (default `canvas`) — writes go
  into the *edited document's* nested ForgeView, so `{$path}` bindings in the
  preview tab react to edits live.
- **Scalars** — `string`/`number`/`bool` get a subscribed `Input` that writes
  `update_state` on `InputEvent::Change`.
- **Objects/arrays** — a `Textarea` pre-filled with pretty JSON; `set` parses
  it back to `update_state`.
- **`bool`** → `Checkbox`, **`number`/`slider`** → `Slider` (min..max from the
  type), **`enum(a | b | c)`** → `Select` (variants parsed from the type
  string), **`color`** → `Input` (hex).
- **"add" row** — two `Input`s write `vars_add.{path,value}`; "+" commits
  `update_state(&path, parsed_value)` — new vars appear instantly.

## `forge build --codegen` — `set_state` boilerplate

`forge build --codegen` emits `pub fn set_state(view, cx)` alongside `pub fn
document()` — it walks the doc's `bind`/`$path`/`{$path}`/`update_state`/
`set_path` references to collect every state path, then emits
`view.set_state(json!({...}))` with all paths initialized to their declared
types (`""`/`0`/`false`/`[]`/`"first-variant"`/`"#000000"`) so the generated
app knows which vars it must wire before `set_document`.
