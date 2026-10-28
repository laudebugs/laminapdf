# E18h — Load test: 50 concurrent reviewers on one document; round close

| Field | Value |
| --- | --- |
| ID | `E18h` |
| Status | `ready` |
| Plan | [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) |
| Round / milestone | Editor phase 6 (E18) — M10 Collaboration |
| Depends on | [E18f](./E18f-sharing-and-notifications.md), [E18g](./E18g-leases-and-base-revisions.md) |
| Blocks | None |
| ADRs | Implements ADR 0012 |
| Branch | `test/e18h-load-test` |
| PR title | `test(collab): load test with 50 concurrent reviewers` |
| Size | M (200–600) |
| PR | — |

## Goal

Plan to-do: simulate 50 reviewers on one document (annotating, replying, filling) against a staging deployment, measure latency, DO CPU and cost, and fix bottlenecks; close E18.

## Context

- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service)

## Scope

**In scope**

- Load-test harness (k6 or a Node client using the real protocol)
- Metrics and cost estimate
- Report in the E18 plan

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** tests/tools
- **`SaveConstraint`:** no new save behaviour beyond the round's: base PDFs immutable; content edits and redactions via lease or server job minting a new base revision
- **Web vs desktop:** n/a
- **Server features:** staging DO
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Harness
- [ ] Run
- [ ] Report
- [ ] Fixes

## Acceptance criteria

- [ ] p95 propagation latency below the target set in E18b with 50 clients for 30 minutes
- [ ] No lost or duplicated records

## Tests

The load test itself.

## Docs to update

- E18 feature plan (results)
- [full-editor §E18](../../plans/full-editor.md#e18-collaboration-hosted-service) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
