# E18d — Collaboration server: Durable Object per document, R2 base revisions, D1 index

| Field | Value |
| --- | --- |
| ID | `E18d` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E18c](./E18c-layer-document-schema.md), [H3](./H3-cloud-document-storage.md), [H4](./H4-document-sync.md) |
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
- **Storage reuses H3/H4, no second store.** Base PDFs live in H3's R2 blob store (`users/{owner}/blobs/{sha256}`, the owner being the user who shared the document); the document index and revision chain are H3's D1 `files` and `revisions` tables and H4's `base_revision` chain, extended by a migration with `members` and a `collab_enabled` flag. E18 adds no separate R2 bucket, D1 document index or revision chain: a new base minted by E18g is just a new H4 revision of the same file, so the owner's other devices sync it through H4's change feed. The DO stores only layer records, history and snapshots ([E18a](E18a-adr-0012-collaboration.md) ADR 0012), keyed by the H3 file ID
- AuthZ via H2 sessions
- Rate limits; 32 MiB message cap respected (no PDF bytes)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `apps/collab`, `laminapdf-sync` (wasm in the DO)
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
- [ ] Storage reuse: a shared document's base is the H3 blob and its revisions are H4 rows (test: share a cloud file, mint a base, see one R2 object per revision and one revision chain)

## Post-merge checks

- [ ] D1 migration (members, `collab_enabled`) applied to production and `apps/collab` deployed by the deploy workflow on `main` (link the run); a WebSocket connection to the production DO succeeds

## Tests

- Worker/DO tests

## Docs to update

- Ops runbook
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
