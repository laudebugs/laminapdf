# E18g — Leases for content edits and redactions; server jobs minting new base revisions

| Field | Value |
| --- | --- |
| ID | `E18g` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E7d](./E7d-select-transform-existing.md), [E10d](./E10d-pades-signing-core.md), [E12d](./E12d-scrub-and-rewrite.md), [E13e](./E13e-server-jobs-infrastructure.md), [E18e](./E18e-client-sync-and-presence.md), [H4](./H4-document-sync.md) |
| ADRs | Implements ADR 0012; follows ADR 0006 §5 (barriers) |
| Branch | `feat/e18g-leases-and-base-revisions` |
| PR title | `feat(collab): leases and new base revisions for non-mergeable edits` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Non-mergeable changes (content edits, page structure if decided, applied redactions, signing) take a lease from the DO or run as a server job that produces a new immutable base revision in R2; clients rebase their layer records onto it.

## Context

- **Design (Paper):** [H06 Shared review](../../design/paper/account-cloud-product.md#h06-shared-review). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- ADR 0012
- [editing notes §3](../../research/architecture/notes/editing-architecture.md#3-collaboration-annotation-layer-sync-crdt-vs-server-authoritative-conflict-semantics)
- What runs under a lease or as a job: content-edit ops from [E7d](E7d-select-transform-existing.md), the `Sign` op from [E10d](E10d-pades-signing-core.md), `ApplyRedactions` from [E12d](E12d-scrub-and-rewrite.md); server jobs reuse the `apps/jobs` Workflow/Container pipeline from [E13e](E13e-server-jobs-infrastructure.md)

## Scope

**In scope**

- Lease protocol
- Base-revision bump and client rebase
- Redaction in collaboration (lease + full rewrite), with **purge of every pre-redaction copy we control**. Base PDFs are content-addressed and immutable, so minting a redacted base leaves the old one, and anything derived from it, recoverable unless it is deleted:
  - **R2:** after the redacted base is committed, every superseded base of that document (all earlier revisions in the H3/H4 revision chain, plus E13e job inputs/outputs for it) is deleted from R2, unless the same hash is still referenced by another document or user (refcount in the D1 `revisions` table; then only this document's reference is dropped). Revision rows are kept as tombstones (hash, size, `purged_at`), never the bytes
  - **DO history:** the document DO deletes its snapshots and op/record history older than the redaction, and rewrites layer records whose values contain redacted text (annotation `/Contents`, replies, field values covered by the redaction) to their scrubbed values, so history cannot replay pre-redaction content; tombstones keep only IDs
  - **Cloud and offline copies:** the DO broadcasts `base-purged { document, purged_hashes }` to connected clients, and the H4 change feed carries the same event, so H4 device caches (desktop cache folder, web OPFS) and E18e offline caches delete the purged revisions and any queued ops against them (queued ops are rebased onto the redacted base or dropped with a report). Clients that reconnect later get the event from the feed before any data
  - Local files the user saved outside lamina's storage are out of our reach; the redaction dialog says so

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `apps/collab`, `laminapdf-sync`
- **`SaveConstraint`:** no new save behaviour beyond the round's: base PDFs immutable; content edits and redactions via lease or server job minting a new base revision
- **Web vs desktop:** identical
- **Server features:** DO, R2
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Leases
- [ ] Base bump
- [ ] Redaction purge: R2 deletion of superseded bases with refcounts, DO history/snapshot purge and record scrub, `base-purged` broadcast and H4 change-feed event, cache invalidation in H4 and E18e clients
- [ ] Tests

## Acceptance criteria

- [ ] Concurrent content edit attempts: one holds the lease, the other is told
- [ ] After a redaction base bump, no client retains unredacted bytes in cache (test, including an offline desktop client that reconnects later)
- [ ] Pre-redaction content is unrecoverable server-side: an integration test plants a secret marker in page text and an annotation, redacts it in a shared document, then lists and reads every R2 object under the document's and owner's prefixes, every DO storage key (records, history, snapshots) and the D1 rows, and finds the marker nowhere; fetching an old revision by hash returns 404
- [ ] A hash shared with another document survives the purge (refcount test)

## Post-merge checks

- [ ] Production deploy of `apps/collab` and the purge job by the deploy workflow on `main` succeeded; a redaction on a staging shared document leaves no pre-redaction object in the production R2 bucket (bucket listing recorded in the done todo)

## Tests

- Integration tests

## Docs to update

- User help
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
