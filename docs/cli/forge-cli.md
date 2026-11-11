# The `forge` CLI

`forge` is the toolchain every document flows through: validate it, reformat
it, scaffold a project around it, compile it to a native app. Run it from the
workspace with `cargo run -p forge-cli -- <cmd>`, or install it with
`cargo install --path crates/forge-cli`.

## Documents

| Command | What it does |
|---|---|
| `forge check <doc>` | Parse and validate a `.forge.yaml` against its base catalog. Prints diagnostics with line/column and hints. Exit 1 on errors. `validate` is an alias. |
| `forge fmt <doc>` | Canonical reformat to stdout. `-w` writes in place. Keeps comments and YAML anchors where the preserving writer can. |
| `forge convert <file>` | Convert between `.forge.yaml` and the JSON document model. `--from yaml\|json`, `--to yaml\|json` (default: the other), `-o <path>` (default stdout. `-` reads stdin). |
| `forge schema` | Print the JSON Schema for the document *model* (the JSON form). |
| `forge schema --yaml [--base <id>]` | Print the JSON Schema for `.forge.yaml` surface syntax, generated from the catalogs — generic, or scoped to one base. These are the schemas editors download and [yaml-language-server](yaml-validation.md) consumes. |

## Projects

Commands that need a document resolve it like this: an explicit
`<doc.forge.yaml>` argument wins. Otherwise the nearest `forge.toml` (searched
up from the doc path, or from the current directory) supplies
[`forge.document`](forge-toml.md). Passing a project *directory* or the
`forge.toml` itself also works.

| Command | What it does |
|---|---|
| `forge new <name> [--base <id>]` | Scaffold `./<name>` — a `forge.toml` and a minimal `app.forge.yaml` pointing at the right `$schema` URL. `--base` defaults to `gpui-kit`, and Forge validates it against the shipped registry. |
| `forge build [doc] [opts]` | Scaffold a minimal app crate under `<doc>/.forge-build/<name>` and compile it. `-o <path>` output binary (default `./<name>`), `--release`, `--codegen` (compile the document to Rust instead of embedding YAML), `--dir <path>` scaffold dir. |
| `forge preview <doc>` | `forge build` then launch the binary — a real preview of the document as an app. |
| `forge package` | `build --release` plus an OS bundle: an AppDir/AppImage on Linux, a `.app` on macOS, NSIS scaffolding on Windows. With `[package] binary = "<crate>"` in `forge.toml` it packages an existing workspace binary instead. |

## Inspecting

| Command | What it does |
|---|---|
| `forge bases` | The registered bases as JSON — id, GPUI crate+version, toolchain, platforms, wasm support, theme, catalog, license, compatible libraries. |
| `forge hooks <doc>` | The hook manifest a document requires — every `on <event>: name()` the app must implement, as JSON. |
| `forge license <doc>` | License report for the document's base and imported libraries. |

## Maintenance

| Command | What it does |
|---|---|
| `forge migrate --base <id> <doc>` | Re-point a document at another base. It rewrites `base:` (preserving the file when the writer can). It then reports elements missing from the target catalog and imports the base doesn't list as compatible. |
| `forge studio` (alias `run`) | Launch [Forge Studio](studio.md) — equivalent to `cargo xtask studio`. |

## Exit codes

`0` success, `1` a command ran and found problems (validation diagnostics,
build failure), `2` bad usage — which prints the full `forge` usage text.
