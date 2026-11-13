# E2p — Source documents, the blob store and the object copier

| Field | Value |
| --- | --- |
| ID | `E2p` |
| Status | `done` |
| Plan | [E2 page organisation §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2a](./E2a-page-organisation-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2p-source-documents-and-copier` |
| PR title | `feat(ops,model): source documents, the blob store and the object copier` |
| Size | L (400–800): the blob store and `add_source` in `laminapdf-ops`, and the `ObjectCopier` in `laminapdf-model`, with their tests and a fuzz target; no op and no binding, which [E2c](../done/E2c-cross-document-pages.md) adds |
| PR | [#509](https://github.com/laudebugs/laminapdf/pull/509) |

## Goal

Give the session the three pieces every cross-document op needs, before any op uses them. A blob store holds other files' bytes by hash, `Document::add_source` opens one as a source, and an `ObjectCopier` deep-copies a page and what it reaches into the target's overlay. Split out of [E2c](../done/E2c-cross-document-pages.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos) so E2c fits one reviewable PR.

## Context

- [E2 plan §2.1, Source documents](../../plans/editor/e2-page-organisation.md#21-ops-and-functions) and [§2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page): how sources are named and what a copy carries
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1: the blob store and `BlobId`; [ADR 0005](../../decisions/0005-document-model.md) §3, §5: fresh object numbers, new IDs
- [05 §3.1 Merge and split](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)
- `BlobId` exists in `crates/laminapdf-ops/src/blob.rs` and no store does

## Scope

**In scope**

- The blob store in `laminapdf_ops::Document`: bytes kept by `BlobId` (SHA-256) for the session, so undo and a restarted worker can replay ops that name them
- `Document::add_source(bytes, password) -> SourceId`: opens the file (decrypting it on read with the password, which never enters an op) and returns the `BlobId` of its bytes; a log that names a source the session lacks fails with `unknown-source` (a new code: one `codes.json` entry and its `docs/errors/` page)
- `ObjectCopier` in `laminapdf-model`: a deep copy across documents with an old → new object map, cycles handled, `Limits` bounding depth and size, and resources shared by several copied pages copied once, matched by stream hash

**Out of scope** (and which todo covers it)

- The ops that copy pages (`InsertPagesFrom`, `ReplacePages`), extract and export → [E2c](../done/E2c-cross-document-pages.md)
- Merge → [E2d](../ongoing/E2d-merge-and-split.md)

**Editor facts**

- **Operations shipped:** none (the pieces E2c's ops use)
- **Crates and packages:** `laminapdf-ops` (blob store, `add_source`), `laminapdf-model` (`ObjectCopier`)
- **`Op` variants:** none
- **`SaveConstraint`:** unchanged; nothing here edits the open document
- **Web vs desktop:** identical (core)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none

## Tasks

- [x] The blob store, keyed by `BlobId`, with `add_source` and the `unknown-source` code
- [x] `ObjectCopier` with the object map, the recursion and size limits, and stream-hash deduplication
- [x] A fuzz target for the copier over corpus pairs

## Acceptance criteria

- [x] `add_source` on the same bytes twice gives the same `SourceId`, on native and wasm32
- [x] Copying a page of each of 10 corpus files into an empty document and saving gives a file that passes `qpdf --check` and renders each page identically to its source in PDFium (pixel diff below threshold)
- [x] Resources shared by several copied pages are copied once (an object-count test)
- [x] A copy that passes `Limits` stops with `LimitExceeded`, leaving the overlay as it was
- [x] An encrypted source opens with its password and its copied objects are written unencrypted

## Post-merge checks

None.

## Tests

- Unit tests in `laminapdf-model` (cycles, shared resources, limits) and `laminapdf-ops` (the store)
- The fuzz target `copy_pages` and its regressions

## Docs to update

- API docs: source documents
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Huge shared resources (fonts) in many sources: hashing may be slow; cache each stream's hash per source

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
