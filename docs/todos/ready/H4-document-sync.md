# H4 — Sync of cloud documents across devices

| Field | Value |
| --- | --- |
| ID | `H4` |
| Status | `ready` |
| Plan | [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features) (cloud storage and sync) |
| Round / milestone | Hosted service foundation (accounts, cloud storage, sync) — before the first round that needs accounts |
| Depends on | [D3b](./D3b-recent-files-and-file-watching.md), [H3](./H3-cloud-document-storage.md) |
| Blocks | [E18d](./E18d-durable-object-server.md), [E18g](./E18g-leases-and-base-revisions.md) |
| Dependency notes | [H3](H3-cloud-document-storage.md), `D3` ([desktop-app §D3](../../plans/desktop-app.md#d3-files): files and watching) |
| ADRs | Implements ADR 0011. Does not follow ADR 0012 ([E18a](E18a-adr-0012-collaboration.md) comes much later): the whole-file `base_revision` chain is defined here, and E18a should reuse it for its base revisions |
| Branch | `feat/h4-document-sync` |
| PR title | `feat(hosted): sync cloud documents across devices` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Keep a user's cloud documents in sync across their devices: new revisions pushed on save, devices notified or polling, background download on desktop, and conflicts (two devices saving from the same base revision) resolved by keeping both (conflict copy) — whole-file revisions, not collaborative merging (E18 covers live co-review).

## Context

- **Design (Paper):** [H03 Cloud documents and upload consent](../../design/paper/account-cloud-product.md#h03-cloud-documents-and-upload-consent), [H04 Sync conflict and offline edits](../../design/paper/account-cloud-product.md#h04-sync-conflict-and-offline-edits). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features)
- [editing notes §3](../../research/architecture/notes/editing-architecture.md#3-collaboration-annotation-layer-sync-crdt-vs-server-authoritative-conflict-semantics)
- [desktop D3](../../plans/desktop-app.md#d3-files): file watching
- [ADR 0005](../../decisions/0005-document-model.md) §3: rebase after save

## Scope

**In scope**

- Revision chain per file with `base_revision` on upload; server rejects stale bases → client creates a conflict copy
- This revision chain (and H3's R2 blob store and D1 index) is the single document store: [E18d](E18d-durable-object-server.md) reuses it for shared documents rather than adding its own, and [E18g](E18g-leases-and-base-revisions.md) adds a `base-purged` change-feed event for redaction. The change feed's event schema is versioned so that event can be added without breaking older clients
- Change feed as a poll endpoint (`GET /sync/changes?since=<cursor>`) on the Worker; no Durable Object and no ADR 0012 dependency
- Desktop: local cache folder, background sync, offline queue
- Web: sync on open/save; OPFS cache

**Out of scope** (and which todo covers it)

- Real-time multi-user review → [E18c](E18c-layer-document-schema.md)
- Push notifications over a per-user Durable Object channel: they would follow ADR 0012 ([E18a](E18a-adr-0012-collaboration.md)), so they are a follow-up after E18a, not part of this todo

**Editor facts**

- **Crates and packages:** `apps/web` Worker, `apps/desktop`, `@laminapdf/ui`
- **`SaveConstraint`:** not applicable (no PDF writes)
- **Web vs desktop:** desktop syncs in the background and offline; web syncs when the document is open
- **Server features:** R2, D1, Workers (polling only)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Revision protocol
- [ ] Conflict copies
- [ ] Change feed
- [ ] Desktop background sync
- [ ] Web cache
- [ ] Tests

## Acceptance criteria

- [ ] Two-device test: edit on A, see on B; concurrent edits produce a conflict copy and no data loss
- [ ] Offline desktop edits upload on reconnect

## Post-merge checks

- [ ] D1 migration (revision chain, change-feed cursor) applied to production and the Worker deployed by the deploy workflow on `main`; a two-device sync against production works (link the run)

## Tests

- Worker tests
- Multi-client integration test
- WebdriverIO offline test

## Docs to update

- User help: sync
- [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features) (cloud storage and sync): tick what this PR completes and point the todo link at `done/`

## Risks and open questions

- Open product question: is sync whole-file only, or should it reuse E18's layer sync for annotations/fields? (recommendation: whole-file here; E18 for live review)

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
