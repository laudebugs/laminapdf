# Linux CI runner (container on the Mac)

The full lane's `check-linux` job (`.github/workflows/full.yml`) runs `cargo xtask check` on Linux ARM64. It runs on a self-hosted runner, `lamina-self-hosted-linux-1` (labels `self-hosted`, `Linux`, `ARM64`), which lives in a Docker container on the same Mac as the two macOS runners. See todo [X5](../../../docs/todos/done/X5-linux-ci-container.md); [ADR 0018 §2](../../../docs/decisions/0018-test-strategy.md#2-ci-lanes) has the lane design.

| File | What it does |
| --- | --- |
| `Dockerfile` | Starts from GitHub's `ghcr.io/actions/actions-runner` image, pinned by digest. Adds `build-essential`, `pkg-config`, `git`, `curl`, `jq`, `ca-certificates`, and the Rust toolchain pinned in `rust-toolchain.toml` (installed with a checksum-verified `rustup-init`). The workflow brings Node, pnpm, `cargo-nextest` and `cargo-deny`. |
| `entrypoint.sh` | Runs inside the container: starts the runner with the JIT config it was given, for one job. |
| `loop.sh` | Runs on the Mac as the supervisor. It builds the image from `main` and, for every job, mints a single-use JIT runner config with `gh` and runs a fresh container (`docker run --rm`). It also sends a heartbeat. |
| `common.sh` | Settings and helpers that the scripts share. |
| `up.sh` | Installs `loop.sh` and `common.sh` as a launchd agent and starts it. |
| `down.sh` | Stops the agent and the container, deregisters the runner and deletes the heartbeat. `--prune` also removes the image. |

## Start it

You need:
- Docker Desktop, running and set to start at login.
- `gh` logged in as an admin of `laudebugs/laminapdf`.
- A checkout of `main` that already contains this directory. The agent builds its image from `main` on GitHub, not from your checkout.

```sh
tools/ci-runner/linux/up.sh
tail -f ~/Library/Logs/lamina-ci-runner-linux.log    # the first start builds the image: a few minutes
gh api repos/laudebugs/laminapdf/actions/runners -q '.runners[] | [.name, .status, ([.labels[].name] | join(","))] | @tsv'
gh variable set LINUX_RUNNER --body true             # once: the switch that lets full.yml schedule check-linux
```

`full.yml`'s `changes` job decides whether each run schedules `check-linux`. It does so only when both of these hold:
- the repository variable `LINUX_RUNNER` is `true` (the owner's switch);
- `LINUX_RUNNER_HEARTBEAT` is less than 15 minutes old. The agent refreshes it every 5 minutes while the runner container is up.

Otherwise the job is skipped, `gate` accepts the skip, and the run shows a warning when the switch is on but the heartbeat is stale. A Mac that is asleep or off, Docker that hasn't started, or a logged-out `gh` therefore stops new runs from waiting on the Linux runner within 15 minutes, with nothing to do by hand.

`up.sh` is safe to run again. It replaces the agent and copies `loop.sh` and `common.sh` to `~/Library/Application Support/lamina-ci-runner-linux/`, so the agent doesn't depend on the worktree you ran it from. When `loop.sh` or `common.sh` changes on `main`, the log says so: rerun `up.sh` from an up-to-date `main`.

- **Log:** `~/Library/Logs/lamina-ci-runner-linux.log`. It has the supervisor's lines, image builds and the runner's output.
- **Agent:** `~/Library/LaunchAgents/dev.lamina.ci-runner-linux.plist`. `launchctl print gui/$(id -u)/dev.lamina.ci-runner-linux` shows its state.
- **After a reboot:** nothing to do. launchd starts the agent at login, the agent waits for Docker Desktop, removes the registration the old container left behind and starts a new container.

## When a run is stuck on check-linux

The heartbeat only protects runs that start after the runner went away. A run that scheduled `check-linux` just before the runner died keeps the job queued, for up to 24 hours. `timeout-minutes` doesn't count time in the queue.

1. Bring the runner back if you can. A queued job is picked up as soon as a runner with the three labels comes online. Check the log, Docker Desktop and `gh auth status`; rerunning `up.sh` is safe.
2. Otherwise cancel the run (`gh run cancel <run-id>`). Once the heartbeat is stale (at most 15 minutes) or the switch is off (`gh variable delete LINUX_RUNNER`), rerun the whole workflow with `gh run rerun <run-id>`, without `--failed`. `changes` has to run again to drop `check-linux`; rerunning only the failed jobs reuses its old output and queues again.

## Stop it

```sh
gh variable delete LINUX_RUNNER            # the switch; the heartbeat would also go stale on its own
tools/ci-runner/linux/down.sh              # agent, container, registration, heartbeat
tools/ci-runner/linux/down.sh --prune      # also the image
```

A job running at that moment is cancelled. If GitHub still lists the runner as busy for more than a minute, `down.sh` warns, and you delete the runner under Settings > Actions > Runners. Docker Desktop's disk image grows with the image and the build cache: `docker system df` shows the usage, and `--prune` plus `docker builder prune` reclaim it.

## Isolation, credentials and caches

Every job gets a new container from the image, and the container mounts no volumes. Nothing a job writes, including into `~/.cargo`, `~/.rustup`, the work directory or the runner's own files, survives into the next job.

Ephemeral runners need a new registration for every job, so something has to hold a credential that can create one. The options:

| Option | Credential, and where it lives | Clean container per job |
| --- | --- | --- |
| Registration token passed at start | Expires after an hour, and an ephemeral runner needs a new one per job, so it doesn't work on its own | — |
| A PAT in the container (`config.sh` or `generate-jitconfig` from inside) | A long-lived token with repository admin rights inside the container, readable by every job's code | No: the container is reused |
| **JIT config minted on the host (chosen)** | The owner's `gh` login, which stays in the Mac's keychain. Each container gets only a single-use config for one runner | Yes |

How the JIT config travels:
1. `loop.sh` calls `POST /repos/laudebugs/laminapdf/actions/runners/generate-jitconfig` with the runner's name, the three labels and the default runner group.
2. It passes the result to Docker as `ACTIONS_RUNNER_INPUT_JITCONFIG`, through the environment. The config is never on a command line, in a file on the Mac or in the log.
3. The runner reads that variable as its `--jitconfig` argument, masks it in its logs and removes it from the environment that job steps inherit.

What a job can still reach: the runner itself writes the config's contents (`.runner`, `.credentials`, `.credentials_rsaparams`) into its directory, and the listener's initial environment is readable in `/proc` by the same user. Job steps run as that user, so job code can read the credentials of its own ephemeral runner. That is inherent to GitHub's runner, and acceptable:
- the credentials belong to one runner that takes one job and which GitHub deletes when the job ends;
- they can't register other runners or act on the repository;
- the container, and the credentials with it, is gone after the job.

Nothing secret is in the repository, the image or a volume.

**Caches.** `check-linux` restores the Rust dependencies' build output and the pnpm store from GitHub's cache. GitHub scopes caches by branch: a PR run can restore `main`'s cache but writes only to its own PR's scope, and `main` never reads a PR's cache. The Rust cache is saved by pushes to `main` only, so what a run on `main` executes comes from `main` alone. PR code can't plant a build script or binary that a later `main` run picks up.

The price of this design is a host process, the launchd agent, rather than Docker's restart policy. The agent also uses the owner's `gh` login: logging `gh` out stops new registrations and heartbeats. The log says so, and `full.yml` stops scheduling the job within 15 minutes. `gh auth login` and the next retry pick it up again.

## Image updates

The agent builds `lamina-ci-runner-linux:local` from `main`'s copies of this directory and `rust-toolchain.toml`. The image is labelled with the commits it came from. Before each job, the agent checks whether either input changed on `main` and rebuilds if so. A build that fails is logged, and the previous image stays in use.

- **Base image:** Dependabot (`docker` in `.github/dependabot.yml`) bumps the pinned image weekly. The bump reaches the runner when it merges, with no rerun of `up.sh`. It isn't built before merge: no CI job builds this image. If the bump breaks the build, the log shows it and the runner keeps the last good image.
- **Runner version:** GitHub stops serving runners more than about a month behind the current release. A runner that is behind updates itself at the start of each job (slower, but it works) until the image catches up.
- **Rust toolchain:** a bump of `rust-toolchain.toml` triggers a rebuild before the next job. Until the rebuild is done, `setup-rust-toolchain` downloads the new toolchain into that job's container.

## Resources and limits

- The container is capped with `--cpus 3 --memory 6g`, so the macOS runners and local work keep most of the 8-core, 16 GB Mac. If the macOS jobs slow down by more than about 20%, lower the cap in `loop.sh` and rerun `up.sh`, or take one macOS runner offline while this one runs.
- The runner takes one job at a time.
- ARM64 only: x86_64-specific code paths aren't covered (see todo X5's risks).
- Self-hosted runners must never run fork PRs. That holds while the repository is private.
