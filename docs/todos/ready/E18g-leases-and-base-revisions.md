# E18g — Leases for content edits and redactions; server jobs minting new base revisions

| Field | Value |
| --- | --- |
| ID | `E18g` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E7d](./E7d-select-transform-existing.md), [E10d](./E10d-pades-signing-core.md), [E12d](./E12d-scrub-and-rewrite.md), [E13e](./E13e-server-jobs-infrastructure.md), [E18e](./E18e-client-sync-and-presence.md) |
| Blocks | [E18h](./E18h-load-test.md) |
| ADRs | Implements ADR 0012; follows ADR 0006 §5 (barriers) |
| Branch | `feat/e18g-leases-and-base-revisions` |
| PR title | `feat(collab): leases and new base revisions for non-mergeable edits` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Non-mergeable changes (content edits, page structure if decided, applied redactions, signing) take a lease from the DO or run as a server job that produces a new immutable base revision in R2; clients rebase their layer records onto it.

## Context

- ADR 0012
- [editing notes §3](../../research/architecture/notes/editing-architecture.md#3-collaboration-annotation-layer-sync-crdt-vs-server-authoritative-conflict-semantics)
- What runs under a lease or as a job: content-edit ops from [E7d](E7d-select-transform-existing.md), the `Sign` op from [E10d](E10d-pades-signing-core.md), `ApplyRedactions` from [E12d](E12d-scrub-and-rewrite.md); server jobs reuse the `apps/jobs` Workflow/Container pipeline from [E13e](E13e-server-jobs-infrastructure.md)

## Scope

**In scope**

- Lease protocol
- Base-revision bump and client rebase
- Redaction in collaboration (lease + full rewrite)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `apps/collab`, `lamina-sync`
- **`SaveConstraint`:** no new save behaviour beyond the round's: base PDFs immutable; content edits and redactions via lease or server job minting a new base revision
- **Web vs desktop:** identical
- **Server features:** DO, R2
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Leases
- [ ] Base bump
- [ ] Tests

## Acceptance criteria

- [ ] Concurrent content edit attempts: one holds the lease, the other is told
- [ ] After a redaction base bump, no client retains unredacted bytes in cache (test)

## Post-merge checks

None.

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
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
