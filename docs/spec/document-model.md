# Forge document model

Status: **draft** (spec version `0.1`). Tracks [#3](https://github.com/skamansam/gpui-forge/issues/3).

The document model is the canonical, format-independent AST. [Forge YAML](forge-yaml.md), JSON
(the same tree as YAML) and XML are its encodings; any document must survive
`YAML -> model -> JSON -> model -> XML -> model -> YAML` without loss. The model is pure data: it knows nothing about GPUI or any base component system.
Whether an element, prop or event *exists* is decided later by validation against the catalog
manifests of the document's base and imported libraries.

## Types

```text
Document
  schema  : String?                // JSON Schema URL for editors (`$schema`)
  version : SpecVersion            // "0.1"
  base    : BaseId?                // "gpui-kit" | "bezel" | "adabraka" | "gpui-0.2"; inherited from forge.toml when absent
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
  style    : OrderedMap<Ident, Value>   // keys defined by the style & layout model (#5)
  events   : [EventBinding]        // ordered
  children : [Node]                // ordered

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
  | Enum(Ident)                    // bare word, e.g. primary, center; the catalog may read it as text
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
5. A key appears at most once in `props` and in `style`; `content` and a prop may not both set the
   element's primary prop (detected at catalog validation, since only the catalog knows which prop
   is primary).
6. `TwoWay` bindings appear only as prop values, never in `style`, `content` or handler args.

Everything else (does `kit.Button` exist, is `variant` a valid prop, is `primary` an allowed enum
value, may a `Text` have children, is the library compatible with the base) is catalog validation
([#10](https://github.com/skamansam/gpui-forge/issues/10), [#92](https://github.com/skamansam/gpui-forge/issues/92)).

## Ordering and identity

- `props`, `style`, `events`, `imports` and `children` are ordered; serializers preserve order so
  hand-edited files produce minimal diffs.
- Nodes without an `id` are identified by their path from the root (`root/2/0`) for tooling
  (inspector selection, error reporting). Studio assigns ids when it needs a stable reference.

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
      Node { element: Button, content: "Sign in", props: { variant: Enum(primary), disabled: Binding(form.invalid, OneWay) },
             events: [click -> sign_in()] },
      Node { element: ely.Badge, content: "beta" },
    ],
  },
}
```

## Open questions

- **Control flow.** Conditional (`if`) and repeated (`for`) nodes are reserved but not in 0.1; they
  depend on the state model in [#18](https://github.com/skamansam/gpui-forge/issues/18).
- **Reusable components / includes.** Referencing another `.forge` file as an element (`use` of a
  local file) is out of scope for 0.1.
- **Expressions.** Bindings are plain paths in 0.1 (no `!`, `==`, arithmetic). Derived values are
  computed by hooks.
