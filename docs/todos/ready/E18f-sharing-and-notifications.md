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

## Acceptance criteria

- [ ] Authorisation tests per role
- [ ] Revoked links stop working immediately

## Tests

- Worker tests
- e2e

## Docs to update

- Privacy page
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
