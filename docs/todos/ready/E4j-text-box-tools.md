# E4j — Text boxes and callouts in the interface

| Field | Value |
| --- | --- |
| ID | `E4j` |
| Status | `ready` |
| Plan | [E4 plan §5](../../plans/editor/e4-markup-and-drawing-annotations.md#5-the-viewer-while-drawing) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md), [E4e](./E4e-freetext-callouts.md), [E4f](./E4f-annotation-tools-ui.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e4j-text-box-tools` |
| PR title | `feat(ui): text boxes and callouts, edited in place` |
| Size | M (200–400): the Text box split tool, in-place editing over the layer, and the text inspector |
| PR | — |

## Goal

The Text box ▾ split tool (text box, callout, typewriter text), editing in place in a `contenteditable` box over the annotation, and the text inspector of C03: font, size, bold, italic, alignment, colours, border and line end.

## Context

- **Design (Paper):** [C03 Free text and callouts](../../design/paper/comment-review.md#c03-free-text-and-callouts). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E4 plan §5](../../plans/editor/e4-markup-and-drawing-annotations.md#5-the-viewer-while-drawing) (text boxes) and [§11](../../plans/editor/e4-markup-and-drawing-annotations.md#11-decisions-on-the-open-questions) (the rich-text subset)

## Scope

**In scope**

- `comment.text-box`, `comment.callout` and `comment.typewriter`, with the split tool button
- Editing in place: the annotation hidden in the render, a `contenteditable` `role="textbox"` over it, typing coalesced into one `UpdateAnnot` per editing session, the box growing as text wraps, side handles for the wrap width, the callout's knee and end handles
- The text inspector with the `annot.*` text commands; ⌘B and ⌘I (Ctrl B, Ctrl I) while editing
- The missing-glyph notice "Some characters use a fallback font" and the note on `/RC` that C03 shows
- The text module loaded before Comment mode opens ([E4 plan §2.2](../../plans/editor/e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated))

**Out of scope** (and which todo covers it)

- System fonts on the desktop → [E6c](E6c-add-text.md)
- The desktop → [E4h](E4h-desktop-annotations.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.06` callouts and text boxes in the interface
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`, `apps/web`
- **`SaveConstraint`:** the banner and gating from E1e
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `D.06` → `round-trip` (web)

## Tasks

- [ ] The split tool and the three text tools
- [ ] Editing in place
- [ ] The text inspector
- [ ] Playwright specs tagged with `D.06`

## Acceptance criteria

- [ ] Playwright in Chromium, Firefox and WebKit: a text box, a callout and typewriter text typed in English and Arabic, made bold, saved and reopened in pdf.js with the same text and look
- [ ] Undo after a typing session removes the whole session in one step
- [ ] axe reports no serious violation while editing

## Post-merge checks

None.

## Tests

- Playwright specs in `apps/web/e2e/`
- Vitest for the editing state machine

## Docs to update

- User help: text boxes and callouts
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- `contenteditable` and the appearance lay out text differently; the box shows the browser's layout while typing and the generated appearance once editing ends, so the PR checks the two stay close at common sizes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
