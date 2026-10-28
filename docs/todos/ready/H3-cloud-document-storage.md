# H3 — Opt-in cloud storage of documents (R2 blobs, D1 index, per-user access)

| Field | Value |
| --- | --- |
| ID | `H3` |
| Status | `ready` |
| Plan | [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features) (cloud storage and sync) |
| Round / milestone | Hosted service foundation (accounts, cloud storage, sync) — before the first round that needs accounts |
| Depends on | [E1e](./E1e-save-and-constraint-banner.md), [H2](./H2-accounts-and-login.md) |
| Blocks | [E18b](./E18b-collaboration-feature-plan.md), [E18f](./E18f-sharing-and-notifications.md), [H4](./H4-document-sync.md) |
| ADRs | Implements ADR 0011 |
| Branch | `feat/h3-cloud-document-storage` |
| PR title | `feat(hosted): opt-in cloud storage for documents` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Let signed-in users store chosen documents in the cloud: per-document opt-in upload to R2 (content-addressed revisions), a D1 index (owner, name, size, revision list), per-user access checks on every request, a cloud files list in the open dialog, open/download/rename/delete, and quotas. Nothing is uploaded unless the user chooses it.

## Context

- [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features)
- [architecture §8 Q2](../../plans/architecture.md#8-open-questions-for-the-team)
- [cloudflare notes §2](../../research/architecture/notes/cloudflare.md#2-storage-and-compute-products-r2-d1-kv-durable-objects-queues-workflows-containers-browser-run-workers-ai) (R2, D1 limits; presigned URLs)
- [editing notes §3](../../research/architecture/notes/editing-architecture.md#3-collaboration-annotation-layer-sync-crdt-vs-server-authoritative-conflict-semantics): base PDFs immutable and content-addressed (shared with E18)

## Scope

**In scope**

- Storage API (`/api/files/*`): create upload (presigned PUT or Worker-proxied stream), commit (hash check), list, get, delete
- R2 layout `users/{id}/blobs/{sha256}`; D1 tables files, revisions, quotas
- Client: "Save to cloud" / "Open from cloud" in the `Platform` adapter as another storage provider
- Deletion semantics (immediate removal from index; R2 delete; D1 Time Travel note)

**Out of scope** (and which todo covers it)

- Sync across devices and conflict handling → [H4](H4-document-sync.md)
- Sharing with others → [E18f](E18f-sharing-and-notifications.md)

**Editor facts**

- **Crates and packages:** `apps/web` Worker, `@lamina/ui`, platform adapters
- **`SaveConstraint`:** saving to the cloud uploads the saved bytes (incremental saves upload a new revision; dedupe by hash)
- **Web vs desktop:** identical; desktop keeps local files as the source of truth unless the user opens the cloud copy
- **Server features:** R2, D1, Workers
- **Capability-matrix rows to flip:** none (not a PDF operation)

## Tasks

- [ ] Schema
- [ ] API with authorisation tests
- [ ] Upload path
- [ ] UI
- [ ] Quotas
- [ ] Privacy page

## Acceptance criteria

- [ ] e2e: upload, list, open, delete on web and desktop
- [ ] Authorisation tests: user A can never read/list/delete user B's files (including guessed hashes)
- [ ] Uploads only after explicit per-document action (Playwright network assertions)

## Tests

- Worker tests
- e2e

## Docs to update

- Privacy page
- User help: cloud files
- [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features) (cloud storage and sync): tick what this PR completes and point the todo link at `done/`

## Risks and open questions

- Open product questions: client-side end-to-end encryption option (forgoes server processing; research suggests offering it) — owner decides; storage quotas and pricing
- Large uploads vs Worker body limits: use presigned multipart uploads

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
