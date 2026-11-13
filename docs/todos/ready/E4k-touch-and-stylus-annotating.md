# E4k — Annotating on phones and tablets

| Field | Value |
| --- | --- |
| ID | `E4k` |
| Status | `ready` |
| Plan | [E4 plan §7](../../plans/editor/e4-markup-and-drawing-annotations.md#7-differences-between-the-shells) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [U3a](../done/U3a-responsive-mobile-and-touch.md), [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md), [E4f](./E4f-annotation-tools-ui.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e4k-touch-and-stylus-annotating` |
| PR title | `feat(ui): annotate with a finger or a pen on phones and tablets` |
| Size | M (200–400): two toolbars, four touch tools and the pointer rules |
| PR | — |

## Goal

The phone's annotate bar and options strip (C08) and the tablet's tool shelf (C09): Pen, Highlighter, Eraser, Lasso, Note and Done, with palm rejection by pointer type, Draw with finger, pressure-varying ink, strokes joined within a second, and the comment sheet for an annotation's note.

## Context

- **Design (Paper):** [C08 Phone annotate](../../design/paper/comment-review.md#c08-phone-annotate), [C09 Tablet stylus](../../design/paper/comment-review.md#c09-tablet-stylus). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E4 plan §7](../../plans/editor/e4-markup-and-drawing-annotations.md#7-differences-between-the-shells) and [§11](../../plans/editor/e4-markup-and-drawing-annotations.md#11-decisions-on-the-open-questions) (ink points and pressure)
- [U3a](../done/U3a-responsive-mobile-and-touch.md): the phone shell and touch metrics

## Scope

**In scope**

- The annotate bar and options strip on phones; the tool shelf on tablets once a pen is seen
- `comment.pen` (an alias of `comment.ink`), `comment.highlighter` (text markup over text, else a translucent ink stroke), `comment.eraser` (removes whole strokes through `UpdateAnnot`, or the annotation with its last stroke), `comment.lasso`, `comment.draw-with-finger`
- Pointer rules: with Draw with finger off, only `pointerType: "pen"` draws, touches pan and pinch, and large contacts during a stroke are ignored; `PointerEvent.pressure` gives each point's width where the device reports it
- Strokes within one second join one Ink annotation; each stroke stays one undo step
- The scroll hint on the page pill for the first three sessions, and the comment sheet (C08)

**Out of scope** (and which todo covers it)

- Replies and status in the comment sheet → [E5g](E5g-comments-panel.md)
- Native apps → after ADR 0025

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.04` freehand ink on touch and pen
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`, `apps/web`
- **`SaveConstraint`:** the banner and gating from E1e
- **Web vs desktop:** the web on phones and tablets; Windows touch screens on the desktop follow the same rules in [E4h](E4h-desktop-annotations.md)
- **Server features:** none
- **Capability-matrix rows to flip:** none (E4f flips the web column; this todo adds the touch tests to it)

## Tasks

- [ ] The annotate bar, the options strip and the tool shelf
- [ ] Pen, Highlighter, Eraser and Lasso
- [ ] Pointer rules and pressure
- [ ] The comment sheet
- [ ] Playwright specs with touch and pen emulation, tagged with `D.04`

## Acceptance criteria

- [ ] Playwright with phone and tablet emulation: a pen stroke draws while a touch scrolls; with Draw with finger on, a finger draws; the eraser removes one stroke of three
- [ ] A synthetic pressure ramp gives a stroke whose width follows it, saved and reopened in pdf.js
- [ ] axe reports no serious violation on the annotate bar and the tool shelf, and every target is at least 44 px

## Post-merge checks

- [ ] Tried by hand on an iPad with an Apple Pencil and an Android phone; findings recorded in the done todo

## Tests

- Playwright specs in `apps/web/e2e/` with touch and pen emulation
- Vitest for the pointer rules

## Docs to update

- User help: annotating on a phone or tablet
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`

## Risks and open questions

- Browsers report pen hover and barrel buttons unevenly; the toggle on the barrel button ships only where the platform reports it

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
