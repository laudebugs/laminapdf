# X5 — Linux CI on a self-hosted runner in a container on the Mac

| Field | Value |
| --- | --- |
| ID | `X5` |
| Status | `ongoing` |
| Plan | [architecture §7 cross-cutting](../../plans/architecture.md#7-to-dos) |
| Round / milestone | Cross-cutting — before B6c's runtime matrix and the first desktop release (Round D) |
| Depends on | [X4](../done/X4-full-lane-gate.md) |
| Blocks | [X6](../ready/X6-windows-ci.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) §2 (CI lanes) |
| Branch | `ci/x5-linux-ci-container` |
| PR title | `ci: Linux check on a self-hosted runner in a container (X5)` |
| Size | S |
| PR | — |

## Goal

Since 2026-10-29 every CI job runs on the two self-hosted macOS ARM64 runners (GitHub-hosted minutes are not used), so `cargo xtask check` no longer runs on Linux. Add a third self-hosted runner, a Linux (ARM64) container on the same Mac, and run the full lane's `cargo xtask check` on it for every code PR and push to `main`, under `gate`. Owner decision (2026-10-29): Linux via a container on the Mac; Windows later ([X6](../ready/X6-windows-ci.md)).

## Context

- [ADR 0018 §2 and its X4 implementation note](../../decisions/0018-test-strategy.md#2-ci-lanes): the full lane, `gate`, the self-hosted move.
- `.github/workflows/full.yml` (suite condition, `gate`) and the runner notes at the top of `ci.yml` (isolate-rust-home, pnpm in `runner.temp`, no rust-cache).
- The Mac: 8 cores, 16 GB, Docker CLI installed; the two macOS runners and local agent work already share it.
- GitHub's runner images and the `actions/runner` container (`ghcr.io/actions/actions-runner`), which run on Linux ARM64.

## Scope

**In scope**

- One Linux runner container on the Mac:
  - Image: the official `ghcr.io/actions/actions-runner` (pinned by digest) plus a small Dockerfile layer for build tools (`build-essential`, `pkg-config`, `git`, `curl`, `jq`); Rust, Node and pnpm come from the workflow as on the Mac runners.
  - Registration: a repository runner named `lamina-self-hosted-linux-1` with labels `self-hosted, Linux, ARM64`, registered from a short-lived registration token (`gh api -X POST repos/<repo>/actions/runners/registration-token`) passed at start; never committed. Ephemeral mode (`--ephemeral`) with a restart loop, so every job starts from a clean container.
  - Limits: `--cpus 3 --memory 6g` so the macOS runners and the owner's work keep headroom; a named volume for the Rust toolchain and cargo registry (`CARGO_HOME`, `RUSTUP_HOME`) so jobs don't reinstall Rust each time.
  - Start/stop scripts under `tools/ci-runner/linux/` (`up.sh`, `down.sh`, `README.md`), no secrets in the repo; a `launchd` plist (optional) or the Docker restart policy to bring it back after a reboot.
- `full.yml`: a `check-linux` job on `[self-hosted, Linux, ARM64]` running `cargo xtask check` with the standard suite condition, added to `gate`'s `needs`.
- Docs: ADR 0018's implementation note, CLAUDE.md's CI paragraph, the `ci.yml`/`full.yml` headers, `tools/ci-runner/linux/README.md`.

**Out of scope**

- Windows ([X6](../ready/X6-windows-ci.md)); x86_64 Linux (ARM64 only for now; note it); moving the nightly lane to Linux.

## Tasks

- [ ] `tools/ci-runner/linux/Dockerfile` (FROM the pinned actions-runner digest + build tools) and `up.sh` / `down.sh` (fetch a registration token with `gh`, `docker run -d --restart unless-stopped --cpus 3 --memory 6g -v lamina-linux-cargo:/home/runner/.cargo -v lamina-linux-rustup:/home/runner/.rustup …`, ephemeral re-registration loop; `down.sh` removes the runner from GitHub and the container).
- [ ] Owner runs `tools/ci-runner/linux/up.sh` once on the Mac (it needs the owner's `gh` login); confirm the runner shows `online` with the three labels.
- [ ] `full.yml`: `check-linux` (checkout, isolate-rust-home if needed, setup-rust-toolchain with `cache: false`, pinned nextest/deny via install-action, pnpm into `runner.temp`, `cargo xtask check`), joined to `gate`.
- [ ] One code PR and one docs-only PR through the new job; record timings.
- [ ] Docs as listed.

## Acceptance criteria

- [ ] A code PR runs `cargo xtask check` on Linux ARM64 (container) and macOS, and `gate` fails if the Linux job fails.
- [ ] A docs-only PR skips `check-linux` and `gate` passes.
- [ ] No secret or token is committed; the runner re-registers after a container restart.
- [ ] The macOS runners' job times don't regress by more than ~20% with the container running (CPU/memory caps hold).

## Post-merge checks

- [ ] The first push to `main` after merge runs `check-linux` green.
- [ ] After a Mac reboot, the Linux runner comes back online without manual steps (or the README says what to run).

## Tests

None beyond the workflow itself (`actionlint`, the two PR runs above).

## Docs to update

ADR 0018 implementation note, CLAUDE.md CI paragraph, `ci.yml`/`full.yml` headers, `tools/ci-runner/linux/README.md`, the architecture plan's to-do list.

## Risks and open questions

- CPU contention on one 8-core Mac: three runners plus local work; the caps above, and dropping to one macOS runner while Linux runs, are the levers.
- ARM64 only: x86_64-specific issues (SIMD paths, `cpufeatures`) aren't covered; revisit with X6 or a hosted x86 job on `main` if they appear.
- Docker on macOS runs a Linux VM; its disk image grows with the cargo volumes; `down.sh` documents pruning.
- Self-hosted runners must never run fork PRs; fine while the repository is private.
