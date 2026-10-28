# E18f — Sharing, roles and comment notifications

| Field | Value |
| --- | --- |
| ID | `E18f` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E18d](./E18d-durable-object-server.md), [H3](./H3-cloud-document-storage.md) |
| Blocks | [E18h](./E18h-load-test.md) |
| ADRs | Implements ADR 0012 and ADR 0011 |
| Branch | `feat/e18f-sharing-and-notifications` |
| PR title | `feat(collab): sharing links, roles and notifications` |
| Size | M (200–600) |
| PR | — |

## Goal

Share a cloud document for review: invites and share links (per ADR 0012's Q6 answer), roles (viewer, commenter, editor), revocation, and optional email notifications for new comments and replies.

## Context

- ADR 0012
- [full-editor: optional Cloudflare server features](../../plans/full-editor.md#optional-cloudflare-server-features)

## Scope

**In scope**

- Sharing API and D1 tables
- Share dialog
- Notification sender (opt-in per user)
- **Revocation reaches open sessions.** Removing a member or a share link updates D1 and then calls the document DO (`revoke { principal | link_id }`); the DO closes every matching WebSocket (hibernated ones included) with close code `4403` ("access revoked"), drops their presence, and from then on re-checks membership on every incoming message and on reconnect, so a client that ignores the close cannot keep writing. The client shows "Your access was removed" and discards unsent ops

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `apps/collab`, `@lamina/ui`
- **`SaveConstraint`:** no new save behaviour beyond the round's: base PDFs immutable; content edits and redactions via lease or server job minting a new base revision
- **Web vs desktop:** identical
- **Server features:** D1, Workers, email provider (decided in E18b)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] API
- [ ] Dialog
- [ ] Notifications
- [ ] DO `revoke` handler closing sockets (code 4403) and per-message membership check; client handling of 4403

## Acceptance criteria

- [ ] Authorisation tests per role
- [ ] Revoked links stop working immediately
- [ ] Revocation closes already-open sessions: with two clients connected (one via a share link, one as an invited member, one of them hibernated), revoking each closes its WebSocket with `4403` within 1 s, a write sent afterwards on a stale socket is rejected, and reconnecting fails (Miniflare test)

## Post-merge checks

- [ ] D1 migrations for the sharing tables applied to production by the deploy workflow on `main`, and the production deploy of `apps/collab` succeeded (link the run)

## Tests

- Worker tests
- e2e

## Docs to update

- Privacy page
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
