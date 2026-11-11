# E1f — Metadata editor UI and in-app capability-matrix page

| Field | Value |
| --- | --- |
| ID | `E1f` |
| Status | `ready` |
| Plan | [E1 editing foundation §2](../../plans/editor/e1-editing-foundation.md#2-the-operations) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1c](../done/E1c-metadata-ops.md), [E1e](./E1e-save-and-constraint-banner.md), [X2](../done/X2-capability-matrix.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1f-metadata-ui-capability-page` |
| PR title | `feat(ui): editable document properties and capability matrix page` |
| Size | M (200–600) |
| PR | — |

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
- e2e round-trip tests for J.01–J.03, J.10 on the web (flipping the web column; the desktop column flips with [E1i](./E1i-desktop-save-in-place.md))

**Out of scope** (and which todo covers it)

- General XMP editor → [E3e](E3e-xmp-editor.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.01` title, author, subject, keywords; `J.02` creation/modification dates; `J.03` producer and creator; `J.10` remove selected metadata
- **Crates and packages:** `@laminapdf/ui`, `apps/web`; `tools/xtask` (JSON output of the matrix if `X2` doesn't emit it)
- **`Op` variants:** uses `SetMetadata`, `RemoveMetadata`
- **`SaveConstraint`:** as E1c; the dialog shows the banner state from E1e
- **Web vs desktop:** web here; the desktop shows the same dialog, from the File menu D4b generates, once E1i saves in place
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `J.01` → `round-trip` (web); `J.02` → `round-trip` (web); `J.03` → `round-trip` (web); `J.10` → `round-trip` (web)

## Tasks

- [ ] Convert the properties dialog to a form with dirty tracking and coalesced transactions
- [ ] Wire conformance warnings and the incremental-history notice
- [ ] Emit matrix JSON from `cargo xtask capabilities --json` (coordinate with `X2`) and render the page with filters
- [ ] Tag e2e tests with capability IDs so the CI gate counts them

## Acceptance criteria

- [ ] e2e on the web: edit each field, save, reopen in the app and pdf.js; values match in Info and XMP
- [ ] The capability page lists every manifest row with four status columns and matches `docs/capabilities/README.md`
- [ ] axe: zero serious violations on the dialog and the page

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

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
