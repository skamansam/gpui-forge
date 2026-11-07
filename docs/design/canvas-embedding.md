# In-canvas foreign-preview embedding — design RFC (#120)

Status: **design decided** — fake-embed (position tracking) first, true
pixel embedding as a later phase.

## Problem

Studio's canvas shows a remote preview frame/card backed by a real
floating host window. #120 wants the foreign preview to *look* embedded:
the host window's pixels inside Studio's canvas area.

## Options evaluated

1. **XEmbed** — X11-only `XReparentWindow` of the host's window into the
   canvas slot. Works on the GPUI-family's Linux X11 platform; fails
   Wayland, macOS, the WASM canvases, ratatui (TUI), and iced (winit's
   XEmbed story is flaky). High fidelity, narrow reach.
2. **Render-to-texture / framebuffer streaming** — the host renders to an
   offscreen buffer and streams frames over the JSON protocol. Universal
   across bases, but per-base offscreen capture is the cost — GPUI's
   `WebPlatform`/`wgpu` can render-to-texture, iced's `Application` can't
   easily, ratatui's `TestBackend` captures trivially. Protocol gets
   heavy (per-frame pixel buffers over JSON-lines needs a side channel).
3. **Fake-embed (position tracking)** — Studio reports the canvas slot's
   screen-space bounds; the host positions its window over it and hides
   its chrome. The window still floats but *appears* embedded. Works on
   every windowed base today; no pixel pipeline at all.

## Decision — fake-embed first, framebuffer as phase 2

**Phase 1: position tracking.** A new `frame_rect` protocol command —
Studio sends `{type:"frame_rect", x,y,w,h}` in screen coordinates on
layout/scroll/resize; each windowed host translates its window's bounds
to match (gpui-family `window.set_bounds`/`WindowOptions`, iced `winit`,
ratatui unaffected — its canvas is Studio's terminal tab). Studio keeps
drawing the frame card's chrome (title/statusbar) so raise/focus still
works — the card just gets a `mode: embedded` where the host window
floats exactly over the card's content area.

**Phase 2: real pixels (per-base, where the toolkit allows).** A
`framebuffer` protocol event carrying an offscreen render — gpui-family
hosts render-to-texture + stream; iced via `winit`'s surface; ratatui's
`TestBackend` already gives text cells. The protocol gains a binary side
channel (shared memory or a second fd — JSON-lines can't carry
megabytes/frame efficiently).

## What Phase 1 needs

- `HostMsg::FrameRect { x, y, w, h }` in `protocol.rs` (screen coords).
- Studio: `cx.observe`/layout tracking on the canvas slot → `send(FrameRect)`.
- Hosts: `on_frame_rect` → `window.set_bounds` (gpui-family), `winit`
  window position (iced), no-op (ratatui).
- Studio's `forge.Preview` card gains `mode: embedded` — chrome stays,
  content area becomes transparent (the host window shows through).

## Constraints

- Wayland: XEmbed is dead; position tracking works only if the
  compositor allows absolute positioning (gpui's Wayland backend may
  not). Fake-embed is an X11/macOS answer; Wayland needs Phase 2.
- macOS: `NSView addSubview:` cross-process doesn't exist — Phase 1
  position tracking is the only option there too.
- WASM canvases (#99) already ARE embedded — the canvas *is* the
  preview. Phase 1/2 don't apply.
