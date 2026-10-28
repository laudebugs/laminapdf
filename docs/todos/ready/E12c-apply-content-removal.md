# E12c — Apply redactions: remove glyphs, paths and image pixels

| Field | Value |
| --- | --- |
| ID | `E12c` |
| Status | `ready` |
| Plan | [full-editor §E12](../../plans/full-editor.md#e12-redaction) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E7c](./E7c-content-ir.md), [E12b](./E12b-mark-and-review.md) |
| Blocks | [E12d](./E12d-scrub-and-rewrite.md) |
| ADRs | Uses ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e12c-apply-content-removal` |
| PR title | `feat(redact): remove content under redaction areas` |
| Size | L (600–1,500) |
| PR | — |

## Goal

`ApplyRedactions` content part: remove show-text glyphs intersecting areas and re-emit remaining glyphs with positions that don't encode removed widths (E12a rule); remove/clip paths; decode, blank and re-encode image pixels (incl. SMask, inline images, shared images copied per page); handle Form XObjects, patterns, Type 3 fonts, text render mode 3 and white text; draw the overlay mark as new content.

## Context

- [04 §10.1 Drawing a black box is not redaction](../../research/pdf/04-interactive-security.md#101-drawing-a-black-box-is-not-redaction) (Bland et al. glyph-position leaks)
- [04 §10.2](../../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do)
- ADR 0022

## Scope

**In scope**

- Removal engine in `lamina-content` over the IR
- Image redaction (JPEG re-encode, Flate, JPX: decode where possible or remove the image with a report)
- Overlay drawing

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.06` permanent redaction
- **Crates and packages:** `lamina-content`, image codecs
- **`Op` variants:** `ApplyRedactions` (barrier, requires rewrite)
- **`SaveConstraint`:** `RequiresRewrite`: a full rewrite, every earlier revision dropped. On signed files the constraint is `Conflict`: the UI names the signatures that will be invalidated and removed (ADR 0007 §3 override); order redact → rewrite → sign
- **Web vs desktop:** identical; very large files may be routed to desktop (web `Limits`)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** G.06 completes in E12d

## Tasks

- [ ] Text removal and repositioning
- [ ] Paths
- [ ] Images
- [ ] XObjects and patterns
- [ ] Overlay

## Acceptance criteria

- [ ] Property tests: for every new op, before save, undo restores the snapshot and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Text under areas is absent from the content streams (token scan) and the remaining glyph positions do not depend on removed widths (test from E12a)
- [ ] Image pixels under areas are blanked (decode-and-check test)

## Post-merge checks

None.

## Tests

- Unit and fixture tests per content kind

## Docs to update

- Security notes: redaction
- [full-editor §E12](../../plans/full-editor.md#e12-redaction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
