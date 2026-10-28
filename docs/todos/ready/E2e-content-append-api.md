# E2e — `lamina-content` append API

| Field | Value |
| --- | --- |
| ID | `E2e` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B5c](./B5c-lamina-facade-and-examples.md), [B9c](./B9c-text-extraction-bindings-and-differential.md), [E2a](./E2a-page-organisation-feature-plan.md) |
| Blocks | [E2f](./E2f-page-geometry.md), [E4a](./E4a-markup-and-drawing-annotations-feature-plan.md), [E5e](./E5e-flatten-and-summary.md), [E6a](./E6a-adding-content-feature-plan.md), [E6b](./E6b-place-images-and-shapes.md), [E8g](./E8g-form-data-and-flatten.md), [E13c](./E13c-ocr-text-layer.md) |
| Dependency notes | [E2a](E2a-page-organisation-feature-plan.md), `B9` ([pdf-library §B9](../../plans/pdf-library.md#b9-text-extraction): content reading and text extraction), `B5` ([pdf-library §B5](../../plans/pdf-library.md#b5-builder-and-the-lamina-crate): builder and `lamina` crate) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md); extends the crate created by ADR 0023 (B9) |
| Branch | `feat/e2e-content-append-api` |
| PR title | `feat(content): append API that wraps and appends page content streams` |
| Size | M (200–600) |
| PR | — |

## Goal

Add the minimal write API to `lamina-content` that never parses existing operators: wrap a page's existing content in `q … Q` by adding new streams to its `/Contents` array, append new streams (e.g. `q <cm> /Fm1 Do Q`), register Form XObjects under unused resource names, and balance `q`/`Q` by a token scan. Resize (E2f), flattening (E5e, E8g) and adding content (E6) use it.

## Context

- [full-editor §6](../../plans/full-editor.md#6-design-summary) (content-stream layer, step 2: Append)
- [library B9](../../plans/pdf-library.md#b9-text-extraction): the lexer used for the token scan
- [02 §2 Content streams](../../research/pdf/02-imaging-model.md#2-content-streams) and [02 §8 Form XObjects](../../research/pdf/02-imaging-model.md#8-form-xobjects)
- ISO 32000-2 §7.8.2 (content streams, `/Contents` array concatenation), §8.4.2 (graphics state stack)

## Scope

**In scope**

- `PageContentWriter` over `CosMut`: `wrap_existing()`, `prepend(stream)`, `append(stream)`, `add_xobject(form) -> Name`, `add_extgstate`, `add_font` with unused-name allocation in the page's (possibly inherited or shared) `/Resources`
- Copy-on-write of shared `/Resources` dictionaries before adding names
- `q`/`Q` balance scan over existing streams (tokens only, string and inline-image aware) and closing `Q`s when unbalanced
- `Op::AppendContent { page, placement: Prepend|Append, form: BlobId|FormSpec, matrix }` (class `Content`)
- Idempotent wrapping: a page wrapped once is not wrapped again (marker via structure, not private keys)

**Out of scope** (and which todo covers it)

- Parsing/editing existing operators → [E7c](E7c-content-ir.md)

**Editor facts**

- **Crates and packages:** `lamina-content` (writer module), `lamina-build` (Form XObject construction), `lamina-ops`
- **`Op` variants:** `AppendContent` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` violates every DocMDP level; blocked unless overridden
- **Web vs desktop:** identical (core)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none directly; enables C.10 (E2f), D.14 (E5e), E.14 (E8g), group B/A additions (E6)

## Tasks

- [ ] Implement the token scan reusing B9's lexer (no interpretation)
- [ ] Resource-name allocator (`Fm1`, `Gs1`, …) avoiding names in inherited resources
- [ ] Copy-on-write for shared resource dicts and shared content streams
- [ ] `AppendContent` op with undo
- [ ] Tests on pages with unbalanced `q`, inline images containing `Q` bytes, shared resources

## Acceptance criteria

- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Appending a filled rectangle to every page of 30 corpus files renders the original pixels unchanged plus the rectangle (PDFium diff) and passes qpdf `--check`
- [ ] Pages with unbalanced `q` are rendered identically before/after wrapping
- [ ] Fuzz target for the token scan runs 10 min without panic

## Tests

- Unit tests and golden outputs in `crates/lamina-content/tests/append.rs`
- Render diff tests with PDFium
- Fuzz target `content_append_scan`

## Docs to update

- rustdoc for the append API
- Note in ADR 0023 (or the B9 crate README) that the crate now has a write module
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Pages whose content ends inside a text object or with an incomplete operator: the scan must detect and wrap safely or refuse with a coded error

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
