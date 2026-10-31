# E5d — XFDF and FDF import/export of annotations

| Field | Value |
| --- | --- |
| ID | `E5d` |
| Status | `ready` |
| Plan | [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E5b](./E5b-comment-threads-and-review.md), [E5c](./E5c-stamps-and-attachments.md) |
| Blocks | [E8g](./E8g-form-data-and-flatten.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5d-xfdf-fdf` |
| PR title | `feat(interchange): XFDF and FDF annotation import and export` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Export annotations to XFDF (and FDF) and import them, matching annotations by `/NM` (with the duplicate-warning rules of ADR 0005 §5), tested against Acrobat-produced fixtures.

## Context

- **Design (Paper):** [C06 Import and export comments](../../design/paper/comment-review.md#c06-import-and-export-comments). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §3.6 FDF and XFDF](../../research/pdf/04-interactive-security.md#36-fdf-and-xfdf)
- ISO 19444-1 (XFDF)
- [ADR 0005](../../decisions/0005-document-model.md) §5 `/NM` index

## Scope

**In scope**

- XFDF writer/reader (XML, safe parser shared with E1b: no DTDs)
- FDF writer/reader (COS syntax via lamina-syntax)
- Import modes: add, replace by `/NM`, merge
- UI: import/export menu items

**Out of scope** (and which todo covers it)

- Form data in FDF/XFDF/JSON → [E8g](E8g-form-data-and-flatten.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.13` import and export annotation data
- **Crates and packages:** new module `lamina-model::interchange` (or `lamina-xfdf` crate if size warrants; decide in E5a)
- **`Op` variants:** `ImportAnnotations` composite op (class `Annotation`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Annotation` is allowed only under DocMDP P3 or when there is no certification signature (approval signatures alone allow annotations); otherwise a violation → warn and gate. Note the Evil Annotation Attack: annotations added after certification are shown as such in the signature panel (E10)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.13` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] XFDF schema coverage for all E4/E5 subtypes
- [ ] FDF
- [ ] Import matching
- [ ] Acrobat fixtures (exported from Acrobat, provenance recorded)

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@laminapdf/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Export → import into a clean copy reproduces annotations (facts and render diff)
- [ ] Acrobat-exported XFDF imports with all supported subtypes; unsupported ones reported

## Post-merge checks

None.

## Tests

- Golden XFDF/FDF
- Acrobat fixture tests
- Fuzz target for the XFDF reader

## Docs to update

- API docs: XFDF
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
