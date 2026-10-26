# ForgeDSL

Status: **draft** (spec version `0.1`). Tracks [#2](https://github.com/skamansam/gpui-forge/issues/2).
ForgeDSL is the hand-written, diff-friendly encoding of the [document model](document-model.md).
File extension: `.forge`, UTF-8.

## At a glance

```forge
forge 0.1
base gpui-kit
use ely = "ely-gpui-components" ^0.4

meta
  title: "Login"

VStack #root
  style
    gap: 12px
    padding: 24px
    align: center

  Text "Welcome back, {$user.name}"
    style
      size: lg

  Input #email
    value: bind $form.email
    placeholder: "Email"

  Button "Sign in"
    variant: primary
    disabled: $form.invalid
    on click: sign_in()

  ely.Badge "beta"   // libraries are addressed by their import alias
```

Design goals, in order: one fact per line (so git diffs touch exactly the lines that changed),
easy to hand-edit, unambiguous to parse without a catalog, and a 1:1 mapping onto the document model.

## Structure

A file is a **header** followed by exactly one **root node**.

- `forge <version>` must be the first non-comment line.
- `base <id>` is optional; when absent the base comes from the project's `forge.toml`.
- `use <alias> = "<library-id>" [<version-req>]` imports a component library under a namespace.
  The base's own library needs no import and has no prefix (`Button`, not `kit.Button`); its
  alias (`kit`, `bezel`, `ada`, `guise`) may still be written explicitly.
- `meta` opens a block of `key: "string"` lines.

### Nodes

A node line is:

```text
Element [#id] [content] [key=value ...]
```

- `Element` is a PascalCase identifier, optionally prefixed by an import alias: `ely.Badge`.
- `#id` names the node (unique per document).
- `content` is a single positional value, usually a string: `Text "Hello"`, `Button "Save"`.
  The catalog maps it to the element's primary prop.
- `key=value` pairs are inline props. They are sugar for short nodes; tooling writes props as
  block lines (below), and the formatter only keeps inline props when a node has no block.

The indented block under a node (2 spaces per level, tabs are an error) holds, in any order:

| Line | Meaning |
|---|---|
| `key: value` | prop |
| `style` + indented `key: value` lines | style & layout properties ([#5](https://github.com/skamansam/gpui-forge/issues/5)) |
| `on <event>: handler(args...)` | event binding; `()` may be omitted when there are no args |
| a node line | child node (children keep their written order) |

The formatter's canonical order is props, `style`, events, then children, separated by a blank
line before the first child.

### Values

| Syntax | Model value |
|---|---|
| `"text"`, `"""multi\nline"""` | String (escapes: `\" \\ \n \t \{ \u{1F600}`) |
| `"Hi {$user.name}"` | Interpolated string; `{` must be escaped as `\{` for a literal brace |
| `12`, `1.5`, `-4`, `12px`, `1.5rem`, `50%`, `1fr` | Number with optional unit |
| `true`, `false` | Bool |
| `primary`, `center` | Enum (bare identifier) |
| `#3b82f6`, `#fff`, `#3b82f680` | Color (only in value position; `#` after an element name is an id) |
| `@colors.primary` | Theme token reference |
| `[8px, 16px]` | List |
| `$form.email`, `$items[0].name` | One-way binding |
| `bind $form.email` | Two-way binding (props only) |

### Comments

`//` to end of line, anywhere except inside strings. Comments are preserved by the formatter
when attached to a node or block line (they're kept as trivia, not in the document model).

## Grammar (EBNF)

Indentation is handled by the lexer, which emits `INDENT` / `DEDENT` tokens (Python-style) and
`NL` at the end of each logical line. Blank and comment-only lines produce no tokens.

```ebnf
file        = header , node ;
header      = version_decl , [ base_decl ] , { use_decl } , [ meta_block ] ;
version_decl= "forge" , VERSION , NL ;
base_decl   = "base" , base_id , NL ;
use_decl    = "use" , IDENT , "=" , STRING , [ VERSION_REQ ] , NL ;
meta_block  = "meta" , NL , INDENT , { IDENT , ":" , STRING , NL } , DEDENT ;

node        = element , [ "#" , IDENT ] , [ value ] , { IDENT , "=" , value } , NL ,
              [ INDENT , { block_item } , DEDENT ] ;
element     = [ IDENT , "." ] , TYPE_IDENT ;
block_item  = prop_line | style_block | event_line | node ;
prop_line   = IDENT , ":" , prop_value , NL ;
style_block = "style" , NL , INDENT , { IDENT , ":" , value , NL } , DEDENT ;
event_line  = "on" , IDENT , ":" , IDENT , [ "(" , [ value , { "," , value } ] , ")" ] , NL ;

prop_value  = value | "bind" , binding ;
value       = STRING | NUMBER | "true" | "false" | IDENT | COLOR | token | list | binding ;
token       = "@" , path ;
list        = "[" , [ value , { "," , value } ] , "]" ;
binding     = "$" , path ;
path        = IDENT , { "." , IDENT | "[" , INTEGER , "]" } ;
base_id     = IDENT , { ( "-" | "." ) , ( IDENT | INTEGER ) } ;   (* gpui-kit, gpui-0.2 *)

TYPE_IDENT  = upper , { letter | digit } ;
IDENT       = ( lower | "_" ) , { letter | digit | "_" | "-" } ;
NUMBER      = [ "-" ] , digits , [ "." , digits ] , [ "px" | "rem" | "%" | "fr" ] ;
COLOR       = "#" , hex{3} | hex{6} | hex{8} ;
STRING      = '"' , { char | escape | "{" , binding , "}" } , '"' | '"""' , ... , '"""' ;
VERSION     = digits , "." , digits ;
VERSION_REQ = semver requirement, e.g. ^0.4, >=1.2, =0.3.9 ;
```

Keywords (`forge`, `base`, `use`, `meta`, `style`, `on`, `bind`, `true`, `false`, and reserved
`if`, `for`, `else`, `slot`) cannot be used as prop, style or event names; catalog manifests are
validated against this list.

## Errors

Parsers report errors with `file:line:col`, the offending token, and a fix hint, e.g.

```text
login.forge:14:5: unexpected tab; ForgeDSL indents with 2 spaces
login.forge:9:3: duplicate id `email` (first defined at 6:3)
```

The parser recovers at the next line of equal or lower indentation so editors (Studio, forge-web,
an LSP later) can show all errors at once.

## Mapping to JSON and XML

Each construct maps 1:1 onto the [document model](document-model.md); the JSON and XML encodings
are specified in [#6](https://github.com/skamansam/gpui-forge/issues/6) and
[#7](https://github.com/skamansam/gpui-forge/issues/7). For reference, the `Button` above in JSON:

```json
{
  "element": "Button",
  "content": "Sign in",
  "props": { "variant": { "enum": "primary" }, "disabled": { "bind": "form.invalid" } },
  "events": [{ "event": "click", "handler": "sign_in", "args": [] }]
}
```

## Worked examples

### Settings form with a library import

```forge
forge 0.1
use ely = "ely-gpui-components" ^0.4

VStack #settings
  style
    gap: 16px
    padding: 24px

  Label "Notifications"
    style
      weight: semibold

  HStack
    style
      justify: between
    Text "Email me about updates"
    Switch #email_updates
      checked: bind $settings.email_updates
      on change: save_settings

  Select #theme
    options: ["light", "dark", "system"]
    value: bind $settings.theme

  ely.Stepper #step
    value: $wizard.step
    on change: go_to_step($wizard.id)
```

### Minimal file

```forge
forge 0.1

Text "Hello, world"
```
