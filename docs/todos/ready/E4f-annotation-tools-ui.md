# E4f — Annotation tools and properties panel

| Field | Value |
| --- | --- |
| ID | `E4f` |
| Status | `ready` |
| Plan | [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E1e](./E1e-save-and-constraint-banner.md), [E4c](./E4c-appearances-markup-notes-caret.md), [E4d](./E4d-appearances-ink-and-shapes.md), [E4e](./E4e-freetext-callouts.md) |
| Blocks | [E4g](./E4g-appearance-parity-suite.md), [E5b](./E5b-comment-threads-and-review.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4f-annotation-tools-ui` |
| PR title | `feat(ui): annotation tools and properties panel` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Annotate mode in both apps: toolbar tools (highlight, underline, strike-through, note, caret, ink, rectangle, ellipse, line, arrow, polygon, polyline, text box, callout), selection/move/resize, and a properties panel (colour, opacity, border width/style, line endings, font), with stylus pressure for ink where the platform reports it.

## Context

- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) web vs desktop
- [full-editor §6](../../plans/full-editor.md#6-design-summary) interaction modes

## Scope

**In scope**

- `annotate` mode and one tool module per subtype
- Properties panel with recent colours and defaults per tool (stored per-viewer in local settings)
- Pointer Events pressure → ink width variation (stored as multiple paths or per E4a)
- Keyboard creation and editing paths; ARIA labels; focus order

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.01` highlight / underline / strike through; `D.02` sticky notes and text comments; `D.03` text-correction annotations; `D.04` freehand ink; `D.05` shapes, lines, arrows; `D.06` callouts and text boxes; `B.06` arrows, paths, freehand drawing
- **Crates and packages:** `@lamina/ui`, `@lamina/viewer`
- **`SaveConstraint`:** banner and gating from E1e on signed files
- **Web vs desktop:** identical; pressure only where `PointerEvent.pressure` is meaningful
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.01` → `round-trip` (web, desktop); `D.02` → `round-trip` (web, desktop); `D.03` → `round-trip` (web, desktop); `D.04` → `round-trip` (web, desktop); `D.05` → `round-trip` (web, desktop); `D.06` → `round-trip` (web, desktop); `B.06` → `round-trip` (web, desktop)

## Tasks

- [ ] Tools
- [ ] Selection handles
- [ ] Properties panel
- [ ] e2e per tool tagged with IDs

## Acceptance criteria

- [ ] e2e on both shells: create each annotation type, save, reopen in pdf.js; appearance and properties preserved
- [ ] axe zero serious violations in annotate mode

## Post-merge checks

None.

## Tests

- Playwright and WebdriverIO specs
- Vitest for tool state machines

## Docs to update

- User help: annotating
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
