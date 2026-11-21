# Tickets

Planned work — not bugs, not in progress.

## AI-driven UI automation

Let AI agents drive Forge Studio: inspect the UI, click, type, read state,
take screenshots — so an agent can walk the app, exercise features, and
report/reproduce bugs without a human at the keyboard.

**Approach — debug socket + CLI front-end (recommended).** Studio exposes a
localhost control surface (UNIX socket or loopback HTTP) with a small
automation API:

- `screenshot` → PNG of the current window
- `tree` → serialized element/node dump (the Forge document tree + resolved
  state is already inspectable — `studio.*` state, diagnostics)
- `click` / `type` / `key` → synthetic input at an element id or coordinates
- `query <path>` → read any `studio.*`/`editor.*` state key
- `eval` / `do` → fire a forge event handler (`select`, `change`, …)

A `forge drive <cmd>` subcommand becomes the thin client — any agent (or
shell script, or test harness) drives the app with plain commands:

```sh
forge studio examples/demos/iconify-demo.forge.yaml &   # launch
forge drive screenshot /tmp/shot.png
forge drive click "Tab: Icons"
forge drive query studio.iconset
```

Why this first: no external protocol deps, testable headlessly in CI, and
an MCP server can wrap the same socket later if we want it (MCP is just a
thin protocol shim over the same primitives — not a design commitment now).

**Not now**: embedding an LLM inside studio (model/key management, heavy),
or shipping a raw MCP server directly (extra dep tree, auth surface).

**Open questions**

- Element addressing: node `id`/`key` stability across renders — inspector
  paths may be enough, or expose a stable "automation id".
- Synthetic input path: does gpui-pre let us inject window events, or do we
  drive at the Forge event layer (`fire`) + leave pixel-level clicking to
  coordinate queries?
- Security: bind localhost-only, opt-in flag (`--drive` / env), read-only
  mode vs. full input.
- Scope: drive Forge documents (uniform for ALL forge apps) vs. studio
  chrome specifically — the doc-level layer generalizes to every app built
  on gpui-forge-runtime.
