# E12f — Redaction leak tests across pdf.js, PDFium, pypdf and pdftotext; round close

| Field | Value |
| --- | --- |
| ID | `E12f` |
| Status | `ready` |
| Plan | [full-editor §E12](../../plans/full-editor.md#e12-redaction) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E12d](./E12d-scrub-and-rewrite.md), [E12e](./E12e-sensitive-search.md), [X3a](./X3a-nightly-lane.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `test/e12f-leak-tests` |
| PR title | `test(redact): cross-tool leak and revision-drop suite` |
| Size | M (200–600) |
| PR | — |

## Goal

The plan's proof: for a redaction fixture set (text, images, vectors, annotations, forms, metadata, OCR layer, incremental history, tagged), redacted strings cannot be extracted with pdf.js, PDFium, pypdf or pdftotext, images show blanked pixels, glyph-position leaks are absent, and no previous revision survives; verify E12 rows.

## Context

- [full-editor §E12](../../plans/full-editor.md#e12-redaction)

## Scope

**In scope**

- Fixture set with secret markers
- Four-extractor harness
- Revision check
- Nightly + PR lane

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.06` permanent redaction; `G.07` search sensitive content to redact
- **Crates and packages:** tests
- **`SaveConstraint`:** no new save behaviour beyond the round's: `RequiresRewrite`, no prior revisions kept
- **Web vs desktop:** n/a
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** verifies G.06, G.07

## Tasks

- [ ] Fixtures
- [ ] Harness
- [ ] CI wiring

## Acceptance criteria

- [ ] Zero secret markers recovered by any tool on any fixture
- [ ] Suite runs on every PR touching `lamina-content` or redaction code

## Post-merge checks

- [ ] The leak suite is green in the first scheduled nightly run on `main` (the full corpus × every extraction tool; the PR lane runs a subset). Link the run in a follow-up PR or the done todo

## Tests

The suite itself.

## Docs to update

- `fixtures/README.md`
- [full-editor §E12](../../plans/full-editor.md#e12-redaction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- pdftotext (GPL) runs as an external CLI only (ADR 0019)

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
