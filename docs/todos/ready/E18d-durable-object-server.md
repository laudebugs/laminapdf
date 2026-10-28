# E18d — Collaboration server: Durable Object per document, R2 base revisions, D1 index

| Field | Value |
| --- | --- |
| ID | `E18d` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E18c](./E18c-layer-document-schema.md) |
| Blocks | [E18e](./E18e-client-sync-and-presence.md), [E18f](./E18f-sharing-and-notifications.md) |
| ADRs | Implements ADR 0012 and ADR 0011 |
| Branch | `feat/e18d-durable-object-server` |
| PR title | `feat(collab): Durable Object sync server` |
| Size | L (600–1,500) |
| PR | — |

## Goal

`apps/collab`: a Worker routing authenticated connections to one DO per document with SQLite storage and WebSocket hibernation, LWW per property, deletes win, tombstones, snapshots, R2 content-addressed base PDFs, D1 index of documents and members.

## Context

- ADR 0012
- [cloudflare notes §2](../../research/architecture/notes/cloudflare.md#2-storage-and-compute-products-r2-d1-kv-durable-objects-queues-workflows-containers-browser-run-workers-ai)

## Scope

**In scope**

- DO class with hibernation handlers
- Persistence and snapshotting
- AuthZ via H2 sessions
- Rate limits; 32 MiB message cap respected (no PDF bytes)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `apps/collab`, `lamina-sync` (wasm in the DO)
- **`SaveConstraint`:** no new save behaviour beyond the round's: base PDFs immutable; content edits and redactions via lease or server job minting a new base revision
- **Web vs desktop:** n/a (server)
- **Server features:** Durable Objects, R2, D1
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] DO
- [ ] Persistence
- [ ] AuthZ
- [ ] Tests

## Acceptance criteria

- [ ] Miniflare tests for convergence, reconnect and hibernation resume
- [ ] DO never stores PDF bytes

## Tests

- Worker/DO tests

## Docs to update

- Ops runbook
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
