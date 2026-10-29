# X5 — Linux CI on a self-hosted runner in a container on the Mac

| Field | Value |
| --- | --- |
| ID | `X5` |
| Status | `done` |
| Plan | [architecture §7 cross-cutting](../../plans/architecture.md#7-to-dos) |
| Round / milestone | Cross-cutting — before B6c's runtime matrix and the first desktop release (Round D) |
| Depends on | [X4](./X4-full-lane-gate.md) |
| Blocks | [X6](../ready/X6-windows-ci.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) §2 (CI lanes) |
| Branch | `ci/x5-linux-ci-container` |
| PR title | `ci: Linux check on a self-hosted runner in a container (X5)` |
| Size | S |
| PR | [#45](https://github.com/laudebugs/lamina/pull/45) |

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

- [x] `tools/ci-runner/linux/Dockerfile` (FROM the pinned actions-runner digest + build tools) and `up.sh` / `down.sh` (`down.sh` removes the runner from GitHub and the container). Shipped with a different registration design (see Notes): a host-side launchd agent (`loop.sh`) mints a single-use JIT runner config with `gh` per job and runs a fresh `docker run --rm` container (`--cpus 3 --memory 6g`, volumes `lamina-linux-cargo` and `lamina-linux-rustup`), instead of a `--restart unless-stopped` container re-registering itself
- [x] Owner runs `tools/ci-runner/linux/up.sh` once on the Mac: moved to Post-merge checks (it needs the owner's `gh` login and Docker running); the steps are in the PR description and the README
- [x] `full.yml`: `check-linux` (checkout, setup-rust-toolchain with `cache: false`, pinned nextest/deny via install-action, pnpm into `runner.temp`, setup-node, `pnpm install`, `cargo xtask check`), joined to `gate`. No isolate-rust-home: it would move Rust into the throwaway tool cache instead of the volumes. The job also needs the repository variable `LINUX_RUNNER` to be `true`, and `gate` accepts its skip otherwise, so no PR waits on a runner that isn't up
- [x] One code PR and one docs-only PR through the new job; record timings: moved to Post-merge checks (no Linux runner was online before merge)
- [x] Docs as listed

## Acceptance criteria

- [x] A code PR runs `cargo xtask check` on Linux ARM64 (container) and macOS, and `gate` fails if the Linux job fails: checked before merge by a dry run of the gate script on sample `needs` results (Linux failure or cancellation fails it; a skip passes only while `LINUX_RUNNER` isn't `true`); the live run is in Post-merge checks
- [x] A docs-only PR skips `check-linux` and `gate` passes: checked by the same dry run; the live run is in Post-merge checks
- [x] No secret or token is committed; the runner re-registers after a container restart: nothing secret is in the repository, image or volumes, and `loop.sh` registers a new ephemeral runner for every container (removing a stale registration of the same name first); the live check is in Post-merge checks
- [ ] The macOS runners' job times don't regress by more than ~20% with the container running (CPU/memory caps hold): moved to Post-merge checks

## Post-merge checks

- [ ] The owner runs `tools/ci-runner/linux/up.sh`; `lamina-self-hosted-linux-1` shows `online` with `self-hosted`, `Linux`, `ARM64`; then `gh variable set LINUX_RUNNER --body true`
- [ ] One code PR runs `check-linux` green (record its time here and in ADR 0018's note), and `gate` fails when it fails; one docs-only PR skips it with `gate` green
- [ ] The macOS runners' job times don't regress by more than ~20% with the container running
- [ ] The first push to `main` after `LINUX_RUNNER` is set runs `check-linux` green
- [ ] After a Mac reboot, the Linux runner comes back online without manual steps (the launchd agent waits for Docker Desktop, which must start at login; the README says what to check)

## Notes

- **Registration.** Ephemeral runners need a new registration for each job. A registration token passed at start expires after an hour and covers one registration, and a PAT inside the container would put a long-lived admin credential where every job can read it. So the credential stays on the host: `loop.sh` (installed by `up.sh` as the launchd agent `dev.lamina.ci-runner-linux`) calls `generate-jitconfig` with the owner's `gh` login and hands the container only that single-use config, through the environment. The cost is a host process instead of Docker's restart policy; launchd's `KeepAlive` and `RunAtLoad` cover crashes and reboots. The trade-off table is in `tools/ci-runner/linux/README.md`.
- **Image.** `ghcr.io/actions/actions-runner:2.338.2` pinned by its index digest; Dependabot's new `docker` entry bumps it, and `up.sh` must be rerun after such a bump.
- **Cold builds.** Only `~/.cargo` and `~/.rustup` persist, so every `check-linux` run builds `target/` from scratch under a 3-CPU cap; the job's timeout is 60 minutes.

## Tests

None beyond the workflow itself (`actionlint`, the two PR runs above).

## Docs to update

ADR 0018 implementation note, CLAUDE.md CI paragraph, `ci.yml`/`full.yml` headers, `tools/ci-runner/linux/README.md`, the architecture plan's to-do list.

## Risks and open questions

- CPU contention on one 8-core Mac: three runners plus local work; the caps above, and dropping to one macOS runner while Linux runs, are the levers.
- ARM64 only: x86_64-specific issues (SIMD paths, `cpufeatures`) aren't covered; revisit with X6 or a hosted x86 job on `main` if they appear.
- Docker on macOS runs a Linux VM; its disk image grows with the cargo volumes; `down.sh` documents pruning.
- Self-hosted runners must never run fork PRs; fine while the repository is private.
