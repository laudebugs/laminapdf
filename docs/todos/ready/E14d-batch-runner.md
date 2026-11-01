# E14d — Batch runner over saved op sequences (local)

| Field | Value |
| --- | --- |
| ID | `E14d` |
| Status | `ready` |
| Plan | [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E6d](./E6d-watermarks-headers-footers.md), [E14a](./E14a-compare-revisions-batch-feature-plan.md) |
| Blocks | [E14e](./E14e-server-batch.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e14d-batch-runner` |
| PR title | `feat(batch): apply saved op sequences to many files` |
| Size | M (200–600) |
| PR | — |

## Goal

Record an op sequence (ADR 0006 JSON with `v` envelope), parameterise it, and apply it to many files: desktop over folders (multi-threaded engine processes), web over selected files (sequential in workers), CLI-friendly in the library; with a per-file report.

## Context

- **Design (Paper):** [X05 Batch runner](../../design/paper/convert-compare-produce.md#x05-batch-runner). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) consequences (batch processing)
- [06 §9](../../research/pdf/06-feature-inventory.md#9-suggested-phasing) phase 6

## Scope

**In scope**

- Op-sequence recorder and editor
- **Batch recipes are document-independent.** Recorded ops name things by `LaminaPdfId` ([ADR 0005](../../decisions/0005-document-model.md)), which only exists in the document they were recorded on, so a recipe is not a replay of op JSON. It is a versioned `Recipe { v, steps: [{ op_template, selector }] }` where each step's targets are **selectors** resolved per input file at run time: `AllPages`, `PageIndex(i)` / `PageRange(a..b)` (0-based, negative = from the end), `PageLabel("iv")` / label ranges, `OddPages`/`EvenPages`, `AllAnnotations { subtype }`, `AllFields { name_glob }`, `Document` (document-level ops). The runner expands each step into concrete ops with that file's IDs; a selector that matches nothing is a per-file warning, not a failure. Recording converts the IDs the user touched into the closest selector (and lets the user edit it)
- **Blob bundling:** steps that reference blobs (stamp images, fonts, watermark Form XObjects; `BlobId`, ADR 0006 §1) carry them in the recipe bundle (`recipe.json` + `blobs/<hash>`), and the runner loads them into each document's blob store before applying, so a recipe saved on one machine runs on another
- Runner with per-file transactions and reports
- Library API and example

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `laminapdf-ops`, `apps/desktop`, `@laminapdf/ui`
- **`SaveConstraint`:** no new save behaviour beyond the round's: read-only views; batch outputs follow each op's rules
- **Web vs desktop:** desktop: folders, parallel; web: selected files, zip output
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** no inventory row (06 §9 batch); documented in the matrix notes

## Tasks

- [ ] Recipe format (versioned, `deny_unknown_fields`) with selectors and blob bundle; JSON samples frozen under `fixtures/recipes/v1/`
- [ ] Recorder (IDs → selectors)
- [ ] Runner (selector expansion per file, blob loading)
- [ ] UI
- [ ] Examples

## Acceptance criteria

- [ ] A recipe recorded on one document (Bates numbering + a watermark image + deleting the last page) runs on 100 *different* corpus files with different page counts, labels and object numbers: every output has the stamp on the selected pages, the watermark blob embedded, the right page removed, and passes `qpdf --check`; the report lists per-file selector matches
- [ ] A recipe bundle copied to another machine (fresh blob store) runs identically (byte-identical outputs)
- [ ] Recipes never contain `LaminaPdfId`s (schema test)
- [ ] A failing file doesn't stop the batch

## Post-merge checks

None.

## Tests

- Integration and e2e

## Docs to update

- User help: batch
- [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
