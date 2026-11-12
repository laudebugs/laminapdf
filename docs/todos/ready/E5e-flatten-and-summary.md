# E5e — Flatten annotations into content and comment summary

| Field | Value |
| --- | --- |
| ID | `E5e` |
| Status | `ready` |
| Plan | [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E2e](../ongoing/E2e-content-append-api.md), [E5b](./E5b-comment-threads-and-review.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e5e-flatten-and-summary` |
| PR title | `feat(ops): flatten annotations and export a comment summary` |
| Size | M (200–600) |
| PR | — |

## Goal

Flatten selected or all annotations: each appearance becomes a Form XObject drawn by an appended `q <cm> /FmN Do Q` stream (E2 append API) with the matrix from §12.5.5 algorithm, then the annotation is removed; and generate a comment summary as a new PDF.

## Context

- **Design (Paper):** [C07 Flatten and comment summary](../../design/paper/comment-review.md#c07-flatten-and-comment-summary). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §2.5 Flattening](../../research/pdf/04-interactive-security.md#25-flattening)
- [06 §10](../../research/pdf/06-feature-inventory.md#10-implications-for-lamina): "flattening without appearances" trap
- ISO 32000-2 §12.5.5 (appearance streams, Algorithm 8.1 equivalent)

## Scope

**In scope**

- `FlattenAnnots { ids | all, include_hidden: false }` op; generate missing appearances first (never flatten without `/AP`)
- `all` means every markup annotation; it never includes `/Widget` annotations (form fields are flattened by `FlattenForm` in [E8g](E8g-form-data-and-flatten.md)), and in particular never the widget of a signature field (`/FT /Sig`, signed or not): flattening it would leave a picture of a signature with no signature behind it and break the signature's `/P`/appearance. Explicit `ids` that name a signature widget are refused with `OpError::SignatureWidget`
- Respect `/F` Hidden/NoView/Print flags and OCG membership
- Comment summary builder (list of comments with page thumbnails) as a new document

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.14` flatten annotations into content; `D.15` delete or summarise annotations
- **Crates and packages:** `laminapdf-content` (append API), `laminapdf-build`, `laminapdf-ops`
- **`Op` variants:** `FlattenAnnots` (class `Content`: one class per op, ADR 0006 §1; `Content` is the more restrictive of the two changes it makes, since it is forbidden at every DocMDP level while removing annotations is allowed at P3)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked unless the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock` (the signature panel, E10h, reports the later content change). Note: flattened annotation objects remain in earlier revisions; the dialog offers a full rewrite
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.14` → `round-trip` (core, pdf, web, desktop); `D.15` → `round-trip` (core, pdf, web, desktop), the row's only flip (E5b shipped the delete half without flipping it)

## Tasks

- [ ] Matrix algorithm
- [ ] Missing-appearance generation
- [ ] Op and undo
- [ ] Summary builder
- [ ] UI actions

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug. Expected for `FlattenAnnots` (class `Content`): violation on P1, P2 and P3 (including P3, where annotation changes alone would be allowed); none on the approval-only fixture
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Flattened pages render pixel-identical (below threshold) to the annotated pages in PDFium and pdf.js
- [ ] A test proves no annotation without `/AP` is flattened to nothing
- [ ] `FlattenAnnots { all }` on a fixture with form fields and a signed and an unsigned signature field leaves every `/Widget` (and every `/FT /Sig` field) in place, and the signature still validates after an incremental save; `ids` naming a signature widget returns `OpError::SignatureWidget`

## Post-merge checks

None.

## Tests

- Render diffs
- Trap test
- e2e

## Docs to update

- User help: flattening
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
