# E18a — ADR 0012: collaboration — Durable Object per document, LWW records, leases

| Field | Value |
| --- | --- |
| ID | `E18a` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E17a](./E17a-advanced-interactions-feature-plan.md), [H1](./H1-auth-spike.md) |
| Blocks | [E18b](./E18b-collaboration-feature-plan.md) |
| ADRs | Writes ADR 0012; builds on ADR 0006 (ops, origins, versioning) and 0005 §5 (IDs) |
| Branch | `docs/e18a-adr-0012-collaboration` |
| PR title | `docs(adr): 0012 collaboration on Durable Objects` |
| Size | M (200–600) |
| PR | — |

## Goal

Decide the collaboration model: one Durable Object per document (SQLite storage, WebSocket hibernation), a layer document of records (annotations by `AnnotId`, field values by fully qualified name, page order by fractional index), last-writer-wins per (record, property), deletes win, explicit page-delete vs annotation policy, leases or server jobs for content edits and redactions that mint a new base revision, base PDFs immutable and content-addressed in R2, no PDF bytes over the socket, version negotiation (ADR 0006 §6), and whether accounts are required (full-editor §9 Q6).

## Context

- [editing notes §3](../../research/architecture/notes/editing-architecture.md#3-collaboration-annotation-layer-sync-crdt-vs-server-authoritative-conflict-semantics)
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service)
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §4–§6
- [full-editor §9](../../plans/full-editor.md#9-open-questions) Q6
- [cloudflare notes §2](../../research/architecture/notes/cloudflare.md#2-storage-and-compute-products-r2-d1-kv-durable-objects-queues-workflows-containers-browser-run-workers-ai) (DO limits and pricing)

## Scope

**In scope**

- ADR 0012 file
- Message protocol sketch
- CRDT alternative considered (Loro/Automerge) for offline-first
- Index updates

**Out of scope** (and which todo covers it)

- Implementation (E18c onwards)

**Editor facts**

- **Crates and packages:** (decision) `apps/collab` Worker + DO, `lamina-sync` (record ↔ op mapping)
- **`SaveConstraint`:** no new save behaviour beyond the round's: base PDFs immutable; content edits and redactions via lease or server job minting a new base revision
- **Web vs desktop:** identical on web and desktop
- **Server features:** DO, R2, D1
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Write the ADR
- [ ] Owner sign-off on Q6 (accounts vs capability links)
- [ ] Index updates

## Acceptance criteria

- [ ] ADR accepted

## Post-merge checks

None.

## Tests

None (decision).

## Docs to update

- `docs/decisions/README.md`
- architecture §3
- `CLAUDE.md`
- full-editor §9 Q6
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
