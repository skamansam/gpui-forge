# Upstream porting tracker

Libraries whose GPUI dependency doesn't match a registered base, plus how
each gets supported. Only libraries that fit **no** base belong here.
Crates on `gpui 0.2` go through the Vanilla GPUI 0.2 base (#96). Crates on
`gpui-pre 0.3.x` work today via the gpui-kit base.

| Library | Upstream GPUI | Path to support | Status |
|---|---|---|---|
| ely-core | gpui-pre 0.3.10 | **done** — forge-lib-ely adapter | shipped |
| gpui-video-player | gpui-pre (feature) | adapter (#77), gpui-kit + gpui02 arms. GStreamer system dep | shipped |
| zorite-markdown/editor | gpui 0.2 | adapter (#78). GPL-3.0 stays opt-in per feature | shipped |
| hig-gpui | gpui-kit 0.7.3 | landed — vendored gpui-kit 0.8 bump + forge-lib-hig (#90) | shipped |
| gpui-flow | Zed git | landed — vendored gpui-pre 0.3.10 retarget + forge-lib-flow (#89) | shipped |
| ferrum-flow | gpui 0.2 | landed — forge-lib-gpui02 `ferrum` feature (#86) | shipped |
| rhythm-gpui | gpui 0.2 | landed — forge-lib-gpui02 `rhythm` feature (#87) | shipped |
| gpui-symbols | gpui 0.2 | landed — forge-lib-gpui02 `symbols` feature (#88) | shipped |
| Base GPUI | Zed git | landed — vendored gpui-pre 0.3.10 retarget + forge-lib-base (#85) | shipped |
| Bezel | Zed git | landed — twin + host + `forge build` (#94) | shipped |
| adabraka-ui | adabraka-gpui 0.5 | landed — nested workspace + vendored fix (#95) | shipped |
| ratatui | — | landed — own base + TUI host (#112) | shipped |
| iced | — | landed — own base + Elm host (#113) | shipped |

**Porting means** a fork or upstream PR moving the crate's `gpui` dep to
`gpui-pre 0.3.10` (the gpui-kit base pin) or confirming it on `gpui 0.2`
(for the Vanilla base). A landed port then follows the adapter checklist
in `docs/format/adapters.md`.

## Vendored upstream bugs

Two bases hit published-crate bugs that Forge patches locally via
`[patch.crates-io]` + `vendor/`:

- **`adabraka-gpui 0.5.1`** — `xcb.setup()` in `platform/linux/global_hotkey.rs`
  fails without `use x11rb::connection::Connection as _;` — vendored at
  `hosts/adabraka/vendor/adabraka-gpui` (#95).
- **`bezel-gpui-web 0.3.21`** — references `wasm_thread::Builder`/`JoinHandle`
  in `src/dispatcher.rs` without declaring the dep — vendored at
  `vendor/bezel-gpui-web` with `wasm_thread = "0.3"` added (#99).

Both upstream repos aren't publicly reachable via `gh` — the fixes should be
upstreamed when the repos resolve.
