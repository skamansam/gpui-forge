# Multi-base architecture — design RFC (#71)

Status: infrastructure shipped, adapters pending.

## Goal

A Forge document renders against a **base component system** — gpui-kit today;
Bezel, adabraka-ui, vanilla GPUI 0.2, ratatui (TUI) and iced are planned.
Third-party **libraries** (gpui-video-player, zorite widgets, Ely components…)
bolt onto a base through adapter crates with their own element namespaces.

## What exists

- **Base selection is a compile-time Cargo feature.** `gpui-forge-runtime` has
  `base-*` features; exactly one must be enabled (compile_error otherwise) and
  each unimplemented base has a `compile_error!` stub pointing at its ticket.
  `base.rs` exposes `BASE_ID` and re-exports the base crate (`kit`).
- **Documents declare a base** via `base: gpui-kit` or inherit it from
  `forge.toml`. Validation rejects unknown bases.
- **Namespaces + imports:** `use:` maps an alias to a library id + version;
  `ely.VideoPlayer` resolves through `Import.alias`. Import aliases are unique
  and cannot shadow the base namespace.
- **Per-library catalogs** (`catalogs/*.catalog.yaml`) describe props, children
  rules and snippets; validation runs across the base + imported catalogs.
- **Per-base runtime builds** (#93): feature-gated compilation, base-guards CI
  job verifies unsupported bases fail fast.

## Decisions

1. **One base per binary.** A compiled app targets exactly one base — the base
   is a Cargo feature, not a runtime switch. Static linking keeps the binary
   small and lets adapters use the base's types directly. Studio and the web
   canvas can embed several runtimes via out-of-process preview hosts (#97).
2. **Adapter contract.** An adapter crate for library `L` on base `B` provides:
   - a **catalog manifest** declaring L's elements, props (with Forge types),
     content roles, children rules, snippets and a `base:` compatibility list;
   - a **render shim** mapping `(element, props, style, children)` to L's
     native components, built against B's component API;
   - optional **theme-token mappings** bridging L's styling to B's theme.
   `forge-spec` validates elements/props from the manifest; the shim only runs
   when the matching base feature is compiled.
3. **Library pinning.** `use:` entries carry semver reqs; `forge.toml`
   `[libraries]` passes them to the generated app's Cargo.toml as real deps.
   The resolved adapter crates' versions get recorded in the build manifest.
4. **Compatibility is a data problem.** Bases and libraries declare supported
   combinations in their catalogs (`base:` lists); `forge check` errors on an
   incompatible document before any build. The compatibility matrix for docs
   (#80) is generated from catalog data, not hand-maintained.
5. **Migration is mechanical.** `forge migrate --base <b>` (#101) rewrites
   `base:`, prunes/re-maps props the target base doesn't support, and emits a
   diagnostics report for elements with no equivalent — it does not guess.

## Open

- **Base manifest format + registry** (#91): today only `BASE_ID` exists;
  the manifest is where per-base metadata (elements, theme keys, platform
  constraints) should live.
- **Adapter SDK** (#81): a template crate + authoring guide; the contract in
  §2 gets codified into `forge-adapter` traits — a `ForgeLibrary` trait (#74)
  exposing `catalog()`, `render(element, ctx)`, `hooks()`.
- **License policy** (#82): GPL-marked libraries (zorite) stay opt-in; the
  compiler emits a license report at build time.
- **Porting tracker** (#83): libraries that fit no existing base get ported
  upstream rather than shimmed forever.
