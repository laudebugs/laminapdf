# E3b — Bookmark editor and named destinations

| Field | Value |
| --- | --- |
| ID | `E3b` |
| Status | `ongoing` |
| Plan | [E3 plan §2.1](../../plans/editor/e3-navigation-attachments-inspection.md#21-ops) ([full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2j](../done/E2j-page-tree-and-tree-writers.md), [E3a](../done/E3a-navigation-attachments-inspection-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e3b-bookmarks-and-destinations` |
| PR title | `feat(ops): outline editing, named destinations and TOC generation` |
| Size | L (400–800): seven ops over one list writer, and the Outline tab's editing |
| PR | [#535](https://github.com/laudebugs/laminapdf/pull/535) |

## Goal

Create and edit nested bookmarks (add, rename, retarget, move, nest, colour and style, open state) and named destinations, in the rail's Outline tab with drag and drop and in the named destinations dialog. The ops and the rules they keep are in the [E3 plan §2.1](../../plans/editor/e3-navigation-attachments-inspection.md#21-ops).

## Context

- **Design (Paper):** [O11 Bookmarks editor](../../design/paper/pages-navigation-extraction.md#o11-bookmarks-editor). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §1.2 Document outline](../../research/pdf/04-interactive-security.md#12-document-outline-1233)
- [04 §1.1 Destinations](../../research/pdf/04-interactive-security.md#11-destinations-1232)
- ISO 32000-2 §12.3.3 (outline), §12.3.2.4 (named destinations)

## Scope

**In scope**

- Ops: `AddOutlineItem`, `UpdateOutlineItem` (title, target, colour `/C`, flags `/F`, open or closed), `MoveOutlineItem`, `DeleteOutlineItem`; `SetNamedDestination`, `RenameNamedDestination`, `DeleteNamedDestination`
- The outline view reads `/C` and `/F`; a name stays in the form it was found in (a string in the name tree, or a name object in the catalog's `/Dests`), and new names go into the tree
- Doubly linked list maintenance (`/First /Last /Next /Prev /Parent /Count`) with tolerant reading of broken links
- Bookmark panel: inline rename, drag to reorder/nest, "set destination to current view"

**Out of scope** (and which todo covers it)

- Link annotations → [E3c](../ready/E3c-links.md)
- The contents page → [E3l](../ready/E3l-contents-page.md)
- Viewer preferences and page mode (`I.11`) → [E3d](../ready/E3d-initial-view-and-threads.md)
- The desktop column → [E3k](../ready/E3k-desktop-navigation-and-export.md)
- TOC from struct-tree headings, if decided → [E15b](../ready/E15b-structure-tree-editing.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `I.01` create/edit bookmarks; nested trees; `I.04` named destinations
- **Crates and packages:** `laminapdf-model` (outline write; name-tree writer from [E2j](../done/E2j-page-tree-and-tree-writers.md)), `laminapdf-ops`, `@laminapdf/ui`
- **`Op` variants:** the outline and destination ops (class `Document`)
- **`SaveConstraint`:** `Free` → incremental; signed: outline and destination changes are class `Document`, a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → warn and gate (override names the signatures). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock`
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `I.01` → `round-trip` (core, pdf, web); `I.04` → `round-trip` (core, pdf, web)

## Tasks

- [ ] Outline writer with `/Count` sign semantics
- [ ] Named-destination ops over [E2j](../done/E2j-page-tree-and-tree-writers.md)'s name-tree writer (E3b does not write its own; E3f uses the same writer)
- [ ] Ops with undo
- [ ] Bookmark panel with DnD and keyboard

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Outline facts (titles, levels, targets) match pdf.js and pypdf after edits on corpus files
- [ ] Playwright: create nested bookmarks, reorder, rename a named destination, save, reopen

## Post-merge checks

None.

## Tests

- Unit tests for list maintenance
- Corpus differential (outline facts)
- Playwright/WebdriverIO specs

## Docs to update

- API docs: outline, destinations
- [full-editor §E3](../../plans/full-editor.md#e3-navigation-attachments-inspection-and-extraction) and the [E3 plan §8](../../plans/editor/e3-navigation-attachments-inspection.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
