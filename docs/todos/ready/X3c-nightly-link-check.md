# X3c — Nightly external link check of the docs

| Field | Value |
| --- | --- |
| ID | `X3c` |
| Status | `ready` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting) |
| Round / milestone | Cross-cutting |
| Depends on | [X3a](../done/X3a-nightly-lane.md) |
| Blocks | None |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) §2 (network-dependent checks run nightly, not in the PR lanes) and §4 (the nightly lane may retry once only to report flakiness) |
| Branch | `ci/x3c-link-check` |
| PR title | `ci: nightly external link check of the docs` |
| Size | S (≈ 100–200 lines) |
| PR | — |

## Goal

Find dead external links in the tracked Markdown every night. `cargo xtask docs-links` ([X1](../done/X1-docs-link-checker.md)) checks only relative links and anchors, because checking URLs needs the network and is flaky, so it can't run in the PR lanes.

## Context

- [X1](../done/X1-docs-link-checker.md) left external URLs out of scope.
- The nightly workflow `.github/workflows/nightly.yml` from [X3a](../done/X3a-nightly-lane.md) runs its jobs as `cargo xtask` tasks and opens a `nightly` issue on failure.

## Scope

**In scope**

- `cargo xtask docs-links --external` (or a pinned `lychee` binary called by xtask) over the tracked Markdown: retries, a per-host rate limit, an allowlist file for URLs known to block bots, and a report of dead links.
- A `links` job in `nightly.yml`. It joins the report job's `needs`, so a dead link opens the nightly issue.

**Out of scope**

- Relative links and anchors: [X1](../done/X1-docs-link-checker.md), already in the fast lane.

## Tasks

- [ ] The external check and its allowlist
- [ ] The nightly job
- [ ] Document it in `CLAUDE.md`

## Acceptance criteria

- [ ] A local run lists every external URL checked and the dead ones; a deliberately dead URL in a scratch file fails it
- [ ] The job runs on the branch through a temporary `push` trigger, removed before merge

## Post-merge checks

- [ ] The first scheduled run reports its result (green, or an issue listing dead links)

## Tests

Unit tests for URL extraction and the allowlist.

## Docs to update

- [ ] `CLAUDE.md`
- [ ] [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting): tick the link-check item

## Risks and open questions

- Sites that rate-limit or block CI runners: keep them on the allowlist with a reason.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
