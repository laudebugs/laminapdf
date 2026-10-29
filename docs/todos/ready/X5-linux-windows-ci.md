# X5 — Linux and Windows CI coverage again

| Field | Value |
| --- | --- |
| ID | `X5` |
| Status | `ready` |
| Plan | [architecture §7 cross-cutting](../../plans/architecture.md#7-to-dos) |
| Round / milestone | Cross-cutting — before the first desktop release (Round D) and before B6c's runtime matrix |
| Depends on | [X4](../done/X4-full-lane-gate.md) |
| Blocks | None |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) §2 (CI lanes) |
| Branch | `ci/x5-linux-windows-ci` |
| PR title | `ci: run the full lane's check on Linux and Windows again (X5)` |
| Size | S |
| PR | — |

## Goal

Since 2026-10-29 every CI job runs on the self-hosted macOS ARM64 runners, because GitHub-hosted minutes on this private repository ran out (billing) and are expensive for macOS and Windows. That dropped the full lane's `cargo xtask check` on Linux and Windows, which X4 added. Bring Linux and Windows back, cheaply enough to keep running on every code PR (or at least on every push to `main`), so OS-specific breakage (path handling, line endings, `pnpm.cmd`, shell differences, case-insensitive file systems) is caught before release.

## Context

- [ADR 0018 §2 and its X4 implementation note](../../decisions/0018-test-strategy.md#2-ci-lanes): the full lane and its required `gate`.
- `.github/workflows/full.yml`: the removed `check` matrix (ubuntu-latest, macos-latest, windows-latest) is in the git history of PR #30.
- The self-hosted runners (`lamina-self-hosted-runner-1/2`) share one Mac; their constraints (isolate-rust-home, pnpm in `runner.temp`, no rust-cache) are documented at the top of `ci.yml`.
- The desktop app ships on Windows and Linux too ([architecture §1](../../plans/architecture.md)), so these OSes need coverage before Round D releases.

## Scope

**In scope**

- Pick how to run Linux and Windows: a self-hosted Linux runner (a VM or a container host) and a self-hosted Windows runner, or GitHub-hosted runners with a spending limit and a narrower trigger (pushes to `main` only, or a `ci:full-os` label), or making the repository public (hosted runners are free for public repositories). The owner decides.
- Add the chosen jobs to `full.yml` with the standard suite condition and to `gate`'s `needs` (or a separate non-required workflow if they run only on `main`).
- Update ADR 0018's implementation note, `CLAUDE.md` and the runner notes in `ci.yml`.

**Out of scope**

- Runtime/browser matrix (B6c), desktop packaging and signing (Round D).

## Tasks

- [ ] Owner decision: self-hosted Linux/Windows runners, hosted runners with a budget and narrower trigger, or a public repository.
- [ ] Add the Linux and Windows `cargo xtask check` jobs to `full.yml` (labels matching the chosen runners; isolate-rust-home on self-hosted).
- [ ] Join them to `gate` (if they run on PRs) and run one code PR and one docs-only PR through them.
- [ ] Update ADR 0018's X4 note ("Fast lane on three OSes"), `CLAUDE.md`'s CI paragraph and the `ci.yml` header.

## Acceptance criteria

- [ ] A code PR runs `cargo xtask check` on Linux, macOS and Windows (or, if the owner chose `main`-only, every push to `main` does), and `gate` reflects the result where required.
- [ ] The docs describe exactly which OSes run on which trigger.

## Post-merge checks

- [ ] The first push to `main` after merge runs the Linux and Windows jobs green.

## Tests

None beyond the workflows themselves (`actionlint`, one PR run per OS).

## Docs to update

ADR 0018 implementation note, `CLAUDE.md` (CI paragraph), `ci.yml`/`full.yml` header comments, the architecture plan's to-do list.

## Risks and open questions

- Cost: GitHub-hosted Windows bills at 2× and macOS at 10× Linux on private repositories; a self-hosted Windows machine needs maintenance.
- Self-hosted runners must never run fork PRs; fine while the repository is private, revisit if it goes public.
