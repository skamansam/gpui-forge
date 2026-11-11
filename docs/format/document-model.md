# Forge document model

Status: **draft** (spec version `0.1`).

The document model is the canonical, format-independent AST. [Forge YAML](forge-yaml.md), JSON
(the same tree as YAML) are its encodings. Any document must survive
`YAML -> model -> JSON -> model -> YAML` without loss. We considered XML and
dropped it — JSON covers the machine-interchange need. The format matrix stays
at one authoring format (YAML) plus one machine format (JSON). The model is pure data: it knows nothing about GPUI or any base component system.
Whether an element, prop or event *exists* is one validation decides later against the catalog
manifests of the document's base and imported libraries.

## Types

```text
Document
  schema  : String?                // JSON Schema URL for editors (`$schema`)
  version : SpecVersion            // "0.1"
  base    : BaseId?                // "gpui-kit" | "bezel" | "adabraka" | "gpui-0.2" | "ratatui" | "iced" | …; inherited from forge.toml when absent
  imports : [Import]               // ordered
  meta    : Map<String, String>    // free-form: title, description, author, ...
  root    : Node

Import
  alias   : Ident                  // namespace used in element names, e.g. "ely"
  library : LibraryId              // catalog manifest id, e.g. "ely-gpui-components"
  version : VersionReq?            // semver requirement, e.g. "^0.4"

Node
  element  : ElementRef
  id       : Ident?                // unique within the document; written Element#id or Element .id
  content  : Value?                // positional value; the catalog maps it to the element's primary prop
  props    : OrderedMap<Ident, Value>
  style    : OrderedMap<Ident, Value>   // keys defined by the style & layout model
  events   : [EventBinding]        // ordered
  children : [Node]                // ordered
  control  : Control?              // `if`/`for`/`with`/`match` — absent on plain elements

// A bare control block (`- if: $cond`) stores a `forge.X` sentinel in
// `element` (`forge.If`/`For`/`With`/`Match`) — it never renders or validates
// as an element; `control` carries the whole meaning. An element carrying
// `control` is a *modified* element (`- Button: …` with `if:`/`for:`/`with:`).

Control
  = If    { when: Value, test: Compare?, otherwise: [Node] }
                                                // `if:` cond, `is:`/`not:`, `else:`
  | For   { source: Value, each: Ident, index: Ident?, key: Path?,
        otherwise: [Node] }                     // `for:` list, `each:`/`index:` vars,
                                                // `key:` item-identity path, `else:` empty
  | With  { source: Value, name: Ident }        // `with:` binding, `as:` var
  | Match { source: Value, cases: [Case], otherwise: [Node] }
                                                // `match:` binding, `case`/`else:` arms

Compare = Is(Value) | Not(Value)
Case    = { value: Value, children: [Node] }    // `case <value>:` arm

ElementRef
  namespace : Ident?               // import alias; None = the base library's namespace
  name      : Ident                // PascalCase, e.g. "Button"

EventBinding
  event   : Ident                  // e.g. "click", "change"
  handler : HandlerRef

HandlerRef
  name : Ident                     // string id resolved by the hook registry at runtime (Phase 5)
  args : [Value]

Value
  = String(text)
  | Interpolated([Segment])        // "Hello, {$user.name}"
  | Number(f64, unit?)             // unit: "px" | "rem" | "%" | "fr"; None = unitless
  | Bool
  | Enum(Ident)                    // bare word, e.g. Primary, center; the catalog may read it as text
  | Color(rgba)                    // #rgb, #rrggbb, #rrggbbaa
  | Token(path)                    // theme token reference, written $theme.colors.primary
  | List([Value])
  | Binding(path, mode)            // mode: OneWay | TwoWay

Segment = Text(text) | Binding(path)
Path    = Ident ("." Ident | "[" integer "]")*   // rooted in the view state, e.g. form.items[0].name
```

## Invariants (checked by `forge-spec` without any catalog)

1. `version` is a spec version this crate understands.
2. Node `id`s are unique across the document.
3. Import aliases are unique and do not shadow the base namespace.
4. `ElementRef.namespace`, when present, names a declared import (or the base's own alias).
5. A key appears at most once in `props` and in `style`. `content` and a prop may not both set the
   element's primary prop. Catalog validation detects this, since only the catalog knows which prop
   is primary.
6. `TwoWay` bindings appear only as prop values, never in `style`, `content` or handler args.

Everything else is catalog validation — element existence (`kit.Button`), prop validity
(`variant`), enum values (`Primary`), child rules (`Text` children), and library compatibility.

## Ordering and identity

- `props`, `style`, `events`, `imports` and `children` stay ordered — serializers preserve order, so
  hand-edited files produce minimal diffs.
- `Node.refname` records the node's `<<: *name` merge source (and `merged`, its
  merge-contributed entry keys). `Document.refs` holds each `name: &name` ref
  block as a head-less node body — edits splice into the block in place.
- For nodes without an `id`, tooling uses the path from the root (`root/2/0`) —
  inspector selection, error reporting. Studio assigns ids when it needs a stable reference.

## Example (as a Rust-ish literal)

```text
Document {
  version: "0.1", base: Some("gpui-kit"),
  imports: [Import { alias: "ely", library: "ely-gpui-components", version: Some("^0.4") }],
  meta: { title: "Login" },
  root: Node {
    element: VStack, id: Some("root"),
    style: { gap: 12px, padding: 24px },
    children: [
      Node { element: Text, content: Interpolated(["Welcome back, ", $user.name]) },
      Node { element: Input, id: Some("email"), props: { value: Binding(form.email, TwoWay), placeholder: "Email" } },
      Node { element: Button, content: "Sign in", props: { variant: Enum(Primary), disabled: Binding(form.invalid, OneWay) },
             events: [click -> sign_in()] },
      Node { element: ely.Badge, content: "beta" },
    ],
  },
}
```

## Mutation API

`Document` exposes index-path mutations used by the Studio's inspector and editor:

- `node_at(path)` / `node_at_mut(path)` — `"0.2.1"` selects `root.children[2].children[1]`
  (`"0"` is the root itself).
- `set_prop(path, name, Option<Value>)` — upsert or remove a prop.
- `set_style(path, name, Option<Value>)` — upsert or remove a `style:` entry.
- `insert_node(path, node, index)` / `remove_node(path)` — the palette/undo ops.
- `set_refname(path, Option<String>)` — assigns the `<<: *name` ref. A fresh name
  becomes a new `name: &name` block. All mutation ops end with `sync_refs()`,
  which pushes merged-facet edits into `refs[name]` and applies them back to
  every `*name` site (the Inspector's refname input rides this).

## Open questions

- **Control flow.** Forge reserves conditional (`if`) and repeated (`for`) nodes but leaves them out of 0.1. They
  depend on the state model.
- **Reusable components / includes.** Referencing another `.forge` file as an element (`use` of a
  local file) is out of scope for 0.1.
- **Expressions.** Bindings are plain paths in 0.1 (no `!`, `==`, arithmetic). Derived values are
  computed by hooks.
