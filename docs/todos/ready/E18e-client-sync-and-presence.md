# E18e — Client sync, presence and offline sync on desktop

| Field | Value |
| --- | --- |
| ID | `E18e` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E18d](./E18d-durable-object-server.md) |
| Blocks | [E18g](./E18g-leases-and-base-revisions.md) |
| ADRs | Implements ADR 0012 |
| Branch | `feat/e18e-client-sync-and-presence` |
| PR title | `feat(collab): client sync, presence and offline queue` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Clients connect to the document's DO, apply remote patches as `Remote` transactions (undo rules from ADR 0006 §4), show presence (cursors, selected annotation), and on desktop queue edits offline and reapply on reconnect (Figma-style refetch + reapply).

## Context

- ADR 0012
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §4

## Scope

**In scope**

- Sync client in the worker/engine
- Presence UI
- Offline queue (desktop)
- Undo interplay tests

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `lamina-sync`, `@lamina/viewer`, `@lamina/ui`, `apps/desktop`
- **`SaveConstraint`:** no new save behaviour beyond the round's: base PDFs immutable; content edits and redactions via lease or server job minting a new base revision
- **Web vs desktop:** identical; desktop works offline and syncs on reconnect
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Client
- [ ] Presence
- [ ] Offline
- [ ] Tests

## Acceptance criteria

- [ ] Two-browser e2e: annotations and field values converge; undo of own change after a remote change behaves per ADR 0006
- [ ] Desktop offline edits sync on reconnect

## Tests

- Playwright multi-context
- WebdriverIO

## Docs to update

- User help: shared review
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
