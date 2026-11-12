# E1f — Metadata editor UI and in-app capability-matrix page

| Field | Value |
| --- | --- |
| ID | `E1f` |
| Status | `done` |
| Plan | [E1 editing foundation §2](../../plans/editor/e1-editing-foundation.md#2-the-operations) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1c](./E1c-metadata-ops.md), [E1e](./E1e-save-and-constraint-banner.md), [X2](./X2-capability-matrix.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1f-metadata-ui-capability-page` |
| PR title | `feat(ui): editable document properties and capability matrix page` |
| Size | M (200–600) |
| PR | [#455](https://github.com/laudebugs/laminapdf/pull/455) |

## Goal

Close round E1: make the C6 document properties dialog editable (title, author, subject, keywords, dates, producer/creator, remove selected metadata) on the web, show PDF/A/UA warnings inline, and add the capability-matrix page to the app so users see what each surface supports.

## Context

- **Design (Paper):** [O04 Document info and XMP editor](../../design/paper/edit-session-organise.md#o04-document-info-and-xmp-editor), [O05 Capability page](../../design/paper/edit-session-organise.md#o05-capability-page). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [web C6](../../plans/web-app.md#c6-navigation-print-and-properties): the read-only properties dialog
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix): the matrix and its columns
- [full-editor §10](../../plans/full-editor.md#10-to-dos) cross-cutting: "Show the capability matrix in the app (E1)"

## Scope

**In scope**

- Editable properties dialog bound to `SetMetadata`/`RemoveMetadata` with validation (date pickers, keywords chips)
- Inline `ConformanceWarning`s and an "edits are kept in earlier revisions" notice for `RemoveMetadata` on incremental saves
- Capability page in `@laminapdf/ui` rendering `docs/capabilities/README.md` data (JSON emitted by `cargo xtask capabilities`) bundled at build time, filterable by group and surface
- e2e round-trip tests for J.01–J.03, J.10 on the web (flipping the web column; the desktop column flips with [E1i](../ready/E1i-desktop-save-in-place.md))

**Out of scope** (and which todo covers it)

- General XMP editor → [E3e](../ready/E3e-xmp-editor.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.01` title, author, subject, keywords; `J.02` creation/modification dates; `J.03` producer and creator; `J.10` remove selected metadata
- **Crates and packages:** `@laminapdf/ui`, `apps/web`; `tools/xtask` (JSON output of the matrix if `X2` doesn't emit it)
- **`Op` variants:** uses `SetMetadata`, `RemoveMetadata`
- **`SaveConstraint`:** as E1c; the dialog shows the banner state from E1e
- **Web vs desktop:** web here; the desktop shows the same dialog, from the File menu D4b generates, once E1i saves in place
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `J.01` → `round-trip` (web); `J.02` → `round-trip` (web); `J.03` → `round-trip` (web); `J.10` → `round-trip` (web)

## Tasks

- [x] Convert the properties dialog to a form with dirty tracking and coalesced transactions
- [x] Wire conformance warnings and the incremental-history notice
- [x] Emit matrix JSON from `cargo xtask capabilities --json` (coordinate with `X2`) and render the page with filters
- [x] Tag e2e tests with capability IDs so the CI gate counts them

## Acceptance criteria

- [x] e2e on the web: edit each field, save, reopen in the app and pdf.js; values match in Info and XMP
- [x] The capability page lists every manifest row with four status columns and matches `docs/capabilities/README.md`
- [x] axe: zero serious violations on the dialog and the page

## Post-merge checks

None.

## Tests

- Playwright e2e tests tagged `J.01`–`J.03`, `J.10`
- Vitest component tests for the dialog

## Docs to update

- User help: editing document properties
- Docs site: link to the capability page
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Matrix JSON must be generated at build time, not fetched (privacy, offline)

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **The dialog** (`@laminapdf/ui`'s `PropertiesDialog` with `editing`): a Description tab with the form and a File tab with R08's facts. Apply makes one edit step, "Document properties changed", of a `setMetadata` op for the changed fields and a `removeMetadata` op for the emptied or removed ones; a file whose XMP can't be read is replaced with `replaceMalformed`. The form reads `@laminapdf/viewer`'s new `client.metadata()`, the metadata as the edits leave it. Encrypted documents and documents whose edits diverged keep the read-only dialog.
- **Coalesced transactions:** each Apply is one transaction. Typing doesn't reach the document until Apply, so there is nothing to coalesce per keystroke.
- **The warnings under a field** are the dialog's own copy of the save planner's two rules (`pdfa-one-author`, `pdfua-title-missing`), since the SDK reports conformance warnings only from a save plan, after an edit. `apps/web/e2e/properties-warnings.e2e.test.ts` holds them to the SDK's on the same edits. `pdfa-info-mismatch` is shown in the PDF/A notice instead, and Apply writes the value shown to both sides.
- **The matrix JSON** is written by `cargo xtask capabilities` on every run, as `docs/capabilities/matrix.json` beside the README, with no `--json` flag; `check docs` and `--check` compare it like the README.
- **TypeScript tags** are collected from the reader's e2e test titles only (`apps/web/e2e/*.e2e.test.ts`, surface `web`, plain `it(`/`test(`). `cargo xtask capabilities` counts them without running them: the reader's e2e suite runs them in Chromium, Firefox and WebKit in the fast lane, whose gate a merge needs.
- **The capability page** is at `/capabilities`, over the open document, and opens from More and the palette (`help.capabilities`). The deviations from O04 and O05 are in their "As built (E1f)" notes.
- **The `pdf` column** of J.01–J.03 and J.10 still shows no evidence: E1c's SDK tests aren't tagged, since TypeScript tags outside the reader's e2e tests aren't collected yet. Adding `packages/sdk` to the tag table would flip it.
- **What a counted TypeScript tag proves:** that the reader's e2e suite, which gates every merge, has a passing test of that title. It can't tell an early `return` from a full run, and a test that passes only on its retry counts too (ADR 0018 §4 reports it as flaky). A tag inside a `describe.skipIf`/`runIf` block isn't detected; the reader's suites use those only around tests that need one browser.
- **User help** has no home yet: the reader has no help pages, and the docs site documents the libraries. The reader's README describes editing document properties for contributors; user help for the reader waits for its own todo.
