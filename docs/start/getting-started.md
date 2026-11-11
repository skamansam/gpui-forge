# Getting started

GPUI Forge is a declarative layer over [GPUI](https://www.gpui.rs/). You write
(or draw) one `.forge.yaml` document. Every tool in the ecosystem can read
it — the web editor, Forge Studio, and the preview hosts. The compiler
turns it into a native Rust app.

## The tools

| Tool | What it does |
|---|---|
| Web app | Browser editor + example showcase at [gpui-forge.rudeboy.dev](https://gpui-forge.rudeboy.dev) — no install |
| Forge Studio | Native workbench: palette, outline, inspector, live preview, themes |
| `forge` CLI | Scaffold, validate, format, build and package apps from documents |
| `forge-preview-<base>` | Out-of-process preview hosts Studio drives over IPC |

All four share the same document format and the same catalogs. A doc that
validates in one place renders the same everywhere its base runs.

## Requirements

- **Rust** via [rustup](https://rustup.rs) — the workspace builds on stable.
- **Node.js 20+** — only for hacking on the site/web editor.
- **GStreamer** — only if you build `forge-lib-video` (see the
  [format reference](forge-yaml.md#base-and-library-prerequisites)).

## Try it in the browser

Open [the editor](https://gpui-forge.rudeboy.dev/editor): click components in
from the palette, edit props in the inspector, and watch the real gpui-kit
renderer paint them. The [examples page](https://gpui-forge.rudeboy.dev/examples)
renders every shipped demo — including ratatui docs drawn as terminal frames.

## Try it locally

```sh
git clone https://github.com/skamansam/gpui-forge
cd gpui-forge

cargo xtask studio                    # Forge Studio (needs a display)

cargo run -p forge-cli -- check examples/gpui-kit-showcase.forge.yaml
```

`cargo run -p forge-cli --` runs the `forge` binary from source. Add an alias
or `cargo install --path crates/forge-cli` if you want `forge` on your PATH.

## Your first project

```sh
forge new hello          # forge.toml + app.forge.yaml
cd hello
forge check app.forge.yaml
forge build              # compiles a native app binary (dev profile)
./hello
```

Or in Studio: **File → New Project…** gives you a scaffolded document with no
folder prompt — the path is only asked for when you save.

## Where next

- [The .forge.yaml format](forge-yaml.md) — the syntax every tool reads
- [Forge Studio](studio.md) — the native workbench
- [Web editor](editor.md) — the browser app
- [forge CLI](forge-cli.md) — every command
