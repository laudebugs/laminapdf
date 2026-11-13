# E6b — Place images, shapes, logos and stamps as page content

| Field | Value |
| --- | --- |
| ID | `E6b` |
| Status | `ready` |
| Plan | [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [B5c](../done/B5c-lamina-facade-and-examples.md), [E2e](../done/E2e-content-append-api.md), [E6a](../ongoing/E6a-adding-content-feature-plan.md) |
| Dependency notes | [E6a](E6a-adding-content-feature-plan.md), [E2e](../done/E2e-content-append-api.md), `B5` ([pdf-library §B5](../../plans/pdf-library.md#b5-builder-and-the-laminapdf-crate): builder and `laminapdf` crate) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e6b-place-images-and-shapes` |
| PR title | `feat(content): add images and vector shapes to existing pages` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Add new content objects to existing pages through the append API: images (JPEG passthrough, PNG with SMask), lines/rectangles/ellipses/polygons, logos/icons/stamps as Form XObjects, with opacity (ExtGState), and transform (move/resize/rotate/crop) **new** objects before and after save.

## Context

- **Design (Paper):** [A02 Place image and shapes](../../design/paper/add-edit-content.md#a02-place-image-and-shapes). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [full-editor §6](../../plans/full-editor.md#6-design-summary) append API
- [ADR 0005](../../decisions/0005-document-model.md) §6: one builder path for new and existing documents
- [02 §7 Images](../../research/pdf/02-imaging-model.md#7-images), [02 §4.2 ExtGState](../../research/pdf/02-imaging-model.md#42-extgstate-dictionaries)

## Scope

**In scope**

- `AddContentObject { page, object: ContentObjectSpec, matrix, opacity, placement }` where `ContentObjectSpec` = Image(BlobId) | Shape(PathSpec, style) | Form(BlobId), defined in `laminapdf-ops` (op payload). The name `ContentObject` is reserved for [E7c](E7c-content-ir.md)'s IR type in `laminapdf-content`, so the two never clash
- `UpdateContentObject` / `DeleteContentObject` for objects lamina added (identity per E6a decision)
- Each object is its own Form XObject with an appended `q cm /FmN Do Q` stream so it can be transformed or removed without parsing existing content
- Crop of new images via clip in the form

**Out of scope** (and which todo covers it)

- Editing pre-existing content → [E7d](E7d-select-transform-existing.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `B.01` insert, delete, replace images; `B.02` resize, rotate, crop, reposition; `B.04` adjust transparency/opacity; `B.05` draw lines, rectangles, circles, polygons; `B.09` add logos, icons, stamps; `B.12` transformations on graphical objects
- **Crates and packages:** `laminapdf-content`, `laminapdf-build`, `laminapdf-ops`
- **`Op` variants:** `AddContentObject`, `UpdateContentObject`, `DeleteContentObject` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked unless the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock` (the signature panel, E10h, reports the later content change)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `B.01` → `write` (core, pdf); `B.02` → `write` (core, pdf); `B.04` → `round-trip` (core, pdf); `B.05` → `round-trip` (core, pdf); `B.09` → `round-trip` (core, pdf); `B.12` → `write` (core, pdf)

## Tasks

- [ ] `ContentObjectSpec` payload type (in `laminapdf-ops`) and builders
- [ ] Identity for added objects per E6a
- [ ] Ops with undo
- [ ] Bindings and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Added objects render identically in pdf.js, PDFium and hayro
- [ ] Original page content renders unchanged (diff outside the added object's bbox is zero)

## Post-merge checks

None.

## Tests

- Render diffs
- Goldens
- Property tests

## Docs to update

- API docs: adding content
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- B.01/B.02/B.12 reach `round-trip` only with E7 (existing objects); here they are `write` for new objects

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
