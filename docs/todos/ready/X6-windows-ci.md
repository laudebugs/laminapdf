# X6 — Windows CI coverage

| Field | Value |
| --- | --- |
| ID | `X6` |
| Status | `ready` |
| Plan | [architecture §7 cross-cutting](../../plans/architecture.md#7-to-dos) |
| Round / milestone | Cross-cutting — before the first Windows desktop release (Round D) |
| Depends on | [X5](../done/X5-linux-ci-container.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) §2 (CI lanes) |
| Branch | `ci/x6-windows-ci` |
| PR title | `ci: Windows check (X6)` |
| Size | S |
| PR | — |

## Goal

Run `cargo xtask check` on Windows again (X4 had it on GitHub-hosted `windows-latest`; it was dropped when CI moved to the self-hosted Mac runners). Parked by the owner on 2026-10-29: pick it up before Windows-specific work (the desktop app's Windows build, signing) or when a Windows machine or budget is available.

## Context

Owner decision, 2026-11-07: use standard GitHub-hosted runners on public repositories. The hosted-runner migration adds required native Rust checks on both Windows architectures. This todo retains the full Rust and TypeScript `cargo xtask check` on Windows; it is not complete until that check runs.

- [ADR 0018 §2 and its X4 implementation note](../../decisions/0018-test-strategy.md#2-ci-lanes); [X5](../done/X5-linux-ci-container.md) (the Linux runner pattern to mirror).
- [Hosted CI runner pricing](../../research/architecture/notes/hosted-ci-runners.md) (2026-10-30): Blacksmith vs CircleCI vs GitHub-hosted Windows and Linux runners, with measured run times and monthly estimates; re-check it before choosing.
- Known Windows-specific code: `pnpm.cmd` lookup in xtask, path portability checks (`cargo xtask check docs`).

## Scope

**In scope**

- Owner decision: a self-hosted Windows runner (a Windows machine or a Windows ARM VM), or GitHub-hosted `windows-latest` with a spending limit, run only on pushes to `main` (or a `ci:windows` label).
- A `check-windows` job in `full.yml` (in `gate` if it runs on PRs; otherwise a non-required job on `main`).
- Docs: ADR 0018 note, CLAUDE.md, workflow headers.

**Out of scope**

- Desktop packaging and signing (Round D).

## Tasks

- [ ] Owner decision on where Windows runs.
- [ ] Add the job; run one code PR (or one push to `main`) through it.
- [ ] Update the docs.

## Acceptance criteria

- [ ] `cargo xtask check` runs on Windows on the agreed trigger, and the docs say which.

## Post-merge checks

- [ ] The first run on the agreed trigger is green.

## Tests

None beyond the workflow.

## Docs to update

ADR 0018 implementation note, CLAUDE.md, workflow headers, the architecture plan's to-do list.

## Risks and open questions

- Cost (GitHub-hosted Windows bills 2× Linux on private repositories) or maintenance (a self-hosted Windows machine).
