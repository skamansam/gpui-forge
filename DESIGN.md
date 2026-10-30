# GPUI Forge: Design Guide

> Raw material goes in. Something made comes out.

This document defines the design philosophy, brand identity, and visual language of **GPUI Forge**. It is the reference for anyone building UI, marketing, or documentation for the app.

---

## 1. Philosophy

### 1.1 The myth behind the name

Hephaestus is the Greek god of the forge, craftsmanship, and metalworking. He is the maker of the gods: he built Achilles' shield, Hermes' winged sandals, and the thrones of Olympus. Unlike the other gods, he is defined by *work*, not by glory. His power is skill applied patiently to difficult material.

GPUI Forge takes that spirit. The app is a workshop where developers shape GPU-accelerated interfaces. It is not a showroom.

### 1.2 Core principles

**1. Craft over spectacle.**
The interface should feel made, not decorated. Every element earns its place the way a tool in a smithy does. Avoid ornament that does not serve function.

**2. Heat means something.**
Warm color (ember orange, forge gold) is a signal, never wallpaper. It marks what is active, hot, or important: the focused element, the running build, the primary action. If everything glows, nothing does.

**3. Solid, honest materials.**
Surfaces feel like metal, stone, and cooled iron: flat, weighty, and dependable. No glassy blur, no floaty gradients. Depth comes from layering and contrast.

**4. Speed is the fire.**
GPUI renders on the GPU, and the app should feel like it. Interactions are immediate. Motion is quick, decisive, and purposeful, like a hammer strike rather than a slow fade.

**5. The tools stay in reach.**
A good smith keeps hammer and tongs within arm's length. Common actions should be one step away, keyboard-first, and never hidden behind unnecessary chrome.

**6. Quiet confidence.**
The tone is that of a master craftsperson: calm, precise, a little dry. Not a hype machine.

### 1.3 What we are not

- Not neon, cyberpunk, or glow-heavy.
- Not cartoonish or fantasy-game styled. The mythology is a reference, not a theme park.
- Not minimalist to the point of coldness. There is warmth in the material.

---

## 2. Logo

### 2.1 Concept

The logo is a **circular medallion**, evoking an ancient coin or shield, the kind of object Hephaestus himself might forge. At its center:

- **A hammer and a pair of tongs, crossed.** These are the two defining tools of the smith god and his most common attributes in classical art. The hammer strikes and shapes; the tongs hold and guide. Together they stand for the two halves of building software: force and precision.
- **A glowing ember held in the tongs.** The ember is the raw idea, the work in progress, and the one point of bright heat in the mark. It is the focal point and the origin of the brand's warm accent color.
- **A dashed inner ring.** A nod to the meander and border bands of Greek decorative art, and a visual echo of rivets on a forged shield.
- **A solid gold outer ring.** Frames the mark and gives it weight, like a struck coin's rim.

### 2.2 Construction

| Element | Description |
|---|---|
| Canvas | Square, 200 × 200 units |
| Medallion | Circle, center (100, 100), radius 96 |
| Outer ring | 5 unit gold stroke on the medallion edge |
| Inner ring | Circle radius 82, 5 unit gold stroke, dashed (12 on, 7 off) |
| Hammer | Rotated +45° around center. Handle in gold, head in pale gold |
| Tongs | Rotated −45° around center. Two gold arms, 6 unit stroke, round caps, converging at the jaws |
| Ember | Two concentric circles at the tongs' tip: orange outer (r 9), yellow core (r 5) |
| Pivot | Small forge-red dot with gold outline at the crossing point |

### 2.3 Reference SVG

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200" role="img" aria-label="GPUI Forge logo">
  <circle cx="100" cy="100" r="96" fill="#7A2E12" stroke="#E8B04A" stroke-width="5"/>
  <circle cx="100" cy="100" r="82" fill="none" stroke="#E8B04A" stroke-width="5" stroke-dasharray="12 7"/>
  <g transform="rotate(45 100 100)">
    <rect x="95" y="64" width="10" height="92" rx="3" fill="#E8B04A"/>
    <rect x="66" y="42" width="68" height="28" rx="4" fill="#F3E3B8"/>
  </g>
  <g transform="rotate(-45 100 100)" fill="none" stroke="#E8B04A" stroke-width="6" stroke-linecap="round">
    <path d="M89 160 L97 70"/>
    <path d="M111 160 L103 70"/>
    <path d="M97 70 L100 56 M103 70 L100 56"/>
  </g>
  <g transform="rotate(-45 100 100)">
    <circle cx="100" cy="50" r="9" fill="#F08A24"/>
    <circle cx="100" cy="50" r="5" fill="#FFD25A"/>
  </g>
  <circle cx="100" cy="100" r="5" fill="#7A2E12" stroke="#E8B04A" stroke-width="2"/>
</svg>
```

### 2.4 Clear space and sizing

- **Clear space:** keep a margin of at least 25% of the logo's diameter on all sides.
- **Minimum size:** 32 px for the full medallion. Below that, use the simplified mark (section 2.6).
- **Recommended sizes:** 16, 32, 64, 128, 256, 512, 1024 px for app icons.
- **Never:** stretch, rotate the whole medallion, add drop shadows or glows, recolor the ember to a non-warm color, or place it on a busy photographic background.

### 2.5 Backgrounds

The medallion carries its own background (forge red), so it works on both light and dark surfaces. On very dark red or brown backgrounds, add the clear space margin plus a thin pale-gold keyline so the edge stays visible.

### 2.6 Simplified and monochrome variants

For tiny sizes (system tray, favicon, 16 to 24 px):

- Drop the dashed inner ring.
- Thicken the tool strokes by about 30%.
- Keep the ember as the single warm accent.

For single-color contexts (print, engraving, watermark):

- Use solid gold `#E8B04A` or solid black on light, solid white on dark.
- Render the ember as a filled circle in the same color, with the medallion as a ring only.

---

## 3. Color

The palette comes from the forge itself: scorched brick, hot metal, cooled steel, and soot.

### 3.1 Brand colors (from the logo)

| Name | Hex | Role |
|---|---|---|
| **Forge Red** | `#7A2E12` | Primary brand color. Medallion field, primary surfaces in branded areas, deep accents |
| **Forge Gold** | `#E8B04A` | Rings, tool handles, borders, icon strokes. The "polished metal" color |
| **Pale Gold** | `#F3E3B8` | Hammer head, highlights, text on Forge Red |
| **Ember Orange** | `#F08A24` | Active and hot state. Primary action accent, focus indicators |
| **Ember Yellow** | `#FFD25A` | The hottest point. Small highlights, success flashes, the core of the ember |

### 3.2 Supporting neutrals (from the earlier forge concepts)

| Name | Hex | Role |
|---|---|---|
| **Soot** | `#1F1A17` | Dark UI background, darkest surface |
| **Steel** | `#B8B2A7` | Secondary text on dark, inactive icons, dividers |
| **Pale Steel** | `#E4DFD5` | Light UI background, primary text on dark |

### 3.3 Proposed extended UI palette

These values are suggestions derived from the brand colors for building a full interface. Adjust them after testing in real UI.

| Name | Hex | Role |
|---|---|---|
| Soot Raised | `#2A2420` | Cards and panels on Soot |
| Soot Border | `#3A322C` | Hairlines and dividers on dark |
| Ash | `#F7F4EE` | Light mode page background |
| Ash Raised | `#FFFFFF` | Light mode cards |
| Forge Red Light | `#9A3F1E` | Hover state for red surfaces |
| Forge Red Dark | `#5A2010` | Pressed state, deep shadows |
| Quench Blue | `#4F8FA8` | Informational messages. Cool contrast to the warm palette |
| Verdigris | `#5E9C7F` | Success, named for oxidized bronze |
| Warning Amber | `#E3A02B` | Warnings (keep distinct from Ember Orange by always pairing with an icon) |
| Slag Red | `#C94A2E` | Errors and destructive actions |

### 3.4 Color usage rules

1. **Warm is active, cool is passive.** Ember Orange and Yellow mark interactive or hot elements. Steel and neutrals handle everything at rest.
2. **One accent at a time.** A screen should have a single primary Ember Orange action. Don't scatter it.
3. **Gold for structure, orange for state.** Forge Gold draws borders and icons. Ember Orange shows focus, selection, and progress.
4. **Dark mode is the default.** A forge is a dark room lit by fire. Soot is the home surface. Light mode is fully supported but treated as the secondary theme.
5. **Contrast is non-negotiable.** Body text must meet WCAG AA (4.5:1). Pale Gold on Forge Red passes. Forge Gold on Soot passes. Do not place Ember Orange text on Forge Red for small text.

### 3.5 Suggested pairings

| Surface | Text | Accent |
|---|---|---|
| Soot `#1F1A17` | Pale Steel `#E4DFD5` | Ember Orange `#F08A24` |
| Soot Raised `#2A2420` | Pale Steel `#E4DFD5` | Forge Gold `#E8B04A` |
| Forge Red `#7A2E12` | Pale Gold `#F3E3B8` | Ember Yellow `#FFD25A` |
| Ash `#F7F4EE` | Soot `#1F1A17` | Forge Red `#7A2E12` |

---

## 4. Typography

Suggested direction (confirm against what GPUI and your target platforms support):

- **Interface text:** a clean, neutral humanist or grotesque sans-serif. Readability and compactness matter more than personality here.
- **Code and technical content:** a high-quality monospace with clear distinction between similar characters (`0`/`O`, `1`/`l`/`I`).
- **Display and headings (sparingly):** a sturdy serif or slab with a classical feel for splash screens, about pages, and marketing. It should echo inscriptions on ancient coins without becoming costume.

Rules:

- Two weights in most UI: regular and medium.
- Sentence case for all labels and headings.
- Avoid all-caps except for very small utility labels.

---

## 5. Shape, Space, and Surface

- **Corners:** modest radii (4 to 8 px for controls, 12 px for cards). Hard enough to feel like metal, soft enough to feel finished. Circles are reserved for the logo and avatars.
- **Borders:** thin (1 px or less), using Soot Border on dark and a low-opacity Steel on light. Use Forge Gold borders only for emphasis or branded containers.
- **Elevation:** express depth with surface color steps (Soot, Soot Raised), not drop shadows or blur.
- **Spacing:** use a consistent scale based on 4 px increments (4, 8, 12, 16, 24, 32, 48).
- **Density:** this is a developer tool. Default to compact, information-dense layouts with comfortable hit targets.

---

## 6. Iconography

- Outline style, 1.5 to 2 px strokes, rounded caps and joins, to match the tongs and hammer in the logo.
- Single color, inheriting from the surrounding text. Use Forge Gold or Ember Orange only for emphasis.
- Favor tool and craft metaphors where natural (hammer for build, tongs for grab and move, anvil for base components), but never at the cost of clarity.

---

## 7. Motion

- **Fast and decisive.** Typical transitions: 100 to 180 ms, ease-out.
- **Strike, don't drift.** Prefer crisp state changes and short, snappy movement over long fades.
- **Heat as feedback.** A brief warm pulse (Ember Orange to Ember Yellow) can mark a successful build or applied change. Keep it subtle and rare.
- **Respect user preferences.** Honor reduced-motion settings by replacing movement with instant or opacity-only changes.

---

## 8. Voice and Tone

- **Plain and precise.** Say what happened and what to do next.
- **Calm craftsperson.** Confident, never boastful. A touch of dry humor is welcome, mythological puns are not required.
- **Light myth, not heavy.** Forge vocabulary can appear in small doses (for example "Forge" for the build action, "Tempering" for an optimization pass), but clarity always wins over theme. Never force a metaphor where a standard term is clearer.

Examples:

| Instead of | Prefer |
|---|---|
| "Oops! Something went wrong!" | "Build failed. 3 errors in `main.rs`." |
| "Awesome! Your masterpiece is ready!" | "Build complete in 2.4 s." |

---

## 9. App Icon Checklist

- [ ] Medallion exported at all required sizes with clean edges
- [ ] Simplified variant used at 24 px and below
- [ ] Verified on light and dark OS backgrounds
- [ ] Ember remains the brightest point at every size
- [ ] Monochrome variant prepared for tray and print
- [ ] Source SVG stored in the repo as the single source of truth

---

## 10. Quick Reference

```
Forge Red     #7A2E12
Forge Gold    #E8B04A
Pale Gold     #F3E3B8
Ember Orange  #F08A24
Ember Yellow  #FFD25A
Soot          #1F1A17
Steel         #B8B2A7
Pale Steel    #E4DFD5
```

**In one line:** a dark workshop, a warm fire, one bright ember, and tools made to be used.
