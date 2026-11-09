# Editing in Studio

Studio edits the document *model* — every palette click, drag, or inspector
edit is a mutation that writes back to the YAML source, with undo/redo on top.

## Palette

The left rail lists every element in the base catalog (and any library the
document imports via `use:`), grouped by section.

- **Click** an element to insert it inside the node selected in the outline.
- **Drag** it onto the outline or the preview to pick the exact drop point —
  dropping on a container inserts inside it, dropping on a leaf lands next to
  it.
- Elements from a `use:` library appear under their `alias.` prefix; placing
  one adds the import line to the document if it isn't there.

## Outline — the Structure group

The node tree of the document. Selecting a row makes it the target for
palette inserts and loads it into the inspector.

- **Move** — the arrow buttons nudge the selected node among its siblings.
- **Reparent** — drag a row onto another node to move it there (cycles are
  refused).
- **Delete** — the trash button (or the outline's own affordances) removes the
  selected node.

## Inspector

Typed editors for the selected node's props and `style:` entries, driven by
the catalog — enum props become dropdowns, colors are validated, unknown props
are flagged. Changes fire `prop_changed`/`style_changed` and land in the
source immediately.

## Source pane

The Source tab is the YAML itself and edits both ways: palette/inspector
mutations rewrite the text (preserving anchors and comments where possible),
and typing in the source re-checks and re-renders on the fly. **File →
Format** runs the canonical formatter over it.

## Diagnostics

Catalog validation runs on every change — the same engine `forge check`
uses. Errors surface as squiggles in the source and in the Diagnostics group
with line, column, and a hint.

## Pick mode

The crosshair button in the Inspector arm turns on pick mode: the next element
you click in the preview becomes the outline selection. In a host preview the
same mode is forwarded to the host process.

## Undo/redo

Every mutation pushes a snapshot of document *and* source before it lands, so
**File → Undo** restores the editor text exactly — not just the model.
