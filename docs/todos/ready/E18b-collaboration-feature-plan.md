# E18b — Collaboration (hosted service): feature plan

| Field | Value |
| --- | --- |
| ID | `E18b` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E2b](./E2b-page-structure-ops.md), [E4b](../ongoing/E4b-annotation-model-and-overlay.md), [E5b](./E5b-comment-threads-and-review.md), [E8c](./E8c-acroform-field-model.md), [E18a](./E18a-adr-0012-collaboration.md), [H2](./H2-accounts-and-login.md), [H3](./H3-cloud-document-storage.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e18b-collaboration-feature-plan` |
| PR title | `docs(editor): E18 feature plan (collaboration (hosted service))` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E18 ([full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service)): shared review of annotations, form values and page order with presence and optional email notifications, on one Durable Object per document; content edits and redactions via leases or server jobs. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e18-collaboration.md`
- Adjustments to the E18 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Crates and packages:** (planned for the round) ADR 0012; `laminapdf-sync`; `apps/collab` Durable Objects
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) base PDFs immutable; content edits and redactions via lease or server job minting a new base revision
- **Web vs desktop:** identical; desktop works offline and syncs on reconnect
- **Server features:** Durable Object per document (SQLite, WebSocket hibernation), R2 base revisions, D1 accounts and sharing
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e18-collaboration.md` (Status: Draft) with: goal; the synced record types (annotations, field values, page order), the ops each maps to, conflict rules per ADR 0012, presence and notification features, and targets (latency, concurrent users)
- [ ] Describe model and crate changes (new typed views, new modules, public API in `laminapdf` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E18c onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service)
- [ ] Collaboration has no inventory rows of its own: record in the plan which existing rows (D.*, E.*, C.04) gain a "shared review" note in the capability matrix

## Acceptance criteria

- [ ] The plan lists every synced record type (annotations and replies, review state, field values, page order) with the existing `Op` variants it maps to (E4b, E5b, E8c, E2b), its conflict rule per ADR 0012, and which edits are not synced as records but go through a lease or a server job (content edits, redaction, signing)
- [ ] The plan names the existing inventory rows (D.*, E.*, C.04) that gain a "shared review" note, and adds no new inventory IDs or manifest rows (collaboration has none)
- [ ] The `SaveConstraint` table covers how leases and server jobs write a new base revision on Free, AppendOnly (signed bases), RequiresRewrite and Conflict documents
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes. If the "shared review" notes are added to `docs/capabilities/manifest.toml` in this PR, `cargo xtask capabilities` (from `X2`) accepts the manifest with no row added and no status changed.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Notification email provider and opt-in rules
- Open question: Retention of shared documents after the last participant leaves
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
