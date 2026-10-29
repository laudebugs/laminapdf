# Linux CI runner (container on the Mac)

The full lane's `check-linux` job (`.github/workflows/full.yml`) runs `cargo xtask check` on Linux ARM64 on a self-hosted runner, `lamina-self-hosted-linux-1` (labels `self-hosted`, `Linux`, `ARM64`), which lives in a Docker container on the same Mac as the two macOS runners. Todo [X5](../../../docs/todos/done/X5-linux-ci-container.md); [ADR 0018 §2](../../../docs/decisions/0018-test-strategy.md#2-ci-lanes) has the lane design.

| File | What it does |
| --- | --- |
| `Dockerfile` | GitHub's `ghcr.io/actions/actions-runner` image, pinned by digest, plus `build-essential`, `pkg-config`, `git`, `curl`, `jq`, `ca-certificates`. Rust, Node and pnpm come from the workflow. |
| `entrypoint.sh` | Inside the container: runs the runner with the JIT config it was given, for one job. |
| `loop.sh` | On the Mac: the supervisor. For every job it mints a single-use JIT runner config with `gh` and runs a fresh container (`docker run --rm`). |
| `up.sh` | Builds the image, installs `loop.sh` as a launchd agent and starts it. |
| `down.sh` | Stops the agent and the container and deregisters the runner; `--prune` also removes the image and volumes. |

## Start it

Needs Docker Desktop (running, and set to start at login) and `gh` logged in as an admin of `laudebugs/lamina`.

```sh
tools/ci-runner/linux/up.sh
gh api repos/laudebugs/lamina/actions/runners -q '.runners[] | [.name, .status, ([.labels[].name] | join(","))] | @tsv'
gh variable set LINUX_RUNNER --body true   # once: lets full.yml schedule check-linux
```

The runner shows up `online` within a minute. Until `LINUX_RUNNER` is `true`, `check-linux` is skipped and `gate` accepts that, so CI never waits on a runner that isn't there. Before shutting the runner down for longer than a moment, unset the variable (`gh variable delete LINUX_RUNNER`), or code PRs wait in the queue until it's back.

`up.sh` is safe to run again (after a Dockerfile change or a Dependabot bump of the base image): it rebuilds the image and replaces the agent. It copies `loop.sh` to `~/Library/Application Support/lamina-ci-runner-linux/`, so the agent doesn't depend on the worktree you ran it from; rerun `up.sh` after `loop.sh` changes.

- **Log:** `~/Library/Logs/lamina-ci-runner-linux.log` (the supervisor's lines and the runner's output).
- **Agent:** `~/Library/LaunchAgents/dev.lamina.ci-runner-linux.plist`; `launchctl print gui/$(id -u)/dev.lamina.ci-runner-linux` shows its state.
- **After a reboot:** nothing to do. launchd starts the agent at login; it waits for Docker Desktop, removes the registration the old container left behind, and starts a new one.

## Stop it

```sh
gh variable delete LINUX_RUNNER            # first, so code PRs don't queue for check-linux
tools/ci-runner/linux/down.sh              # agent, container and registration
tools/ci-runner/linux/down.sh --prune      # also the image and the Rust volumes
```

A job running at that moment is cancelled. Docker Desktop's disk image grows with the volumes (`docker system df`); `--prune` and `docker builder prune` reclaim it.

## How registration works, and why

Ephemeral runners need a new registration for every job, so something has to hold a credential that can create one. The options:

| Option | Credential, and where it lives | Clean container per job |
| --- | --- | --- |
| Registration token passed at start | Expires after an hour, and an ephemeral runner needs a new one per job: doesn't work on its own | — |
| A PAT in the container (`config.sh` or `generate-jitconfig` from inside) | A long-lived token with repository admin rights inside the container, readable by every job's code | No: the container is reused |
| **JIT config minted on the host (chosen)** | The owner's `gh` login, which stays in the Mac's keychain; each container gets only a single-use config for one runner | Yes |

`loop.sh` calls `POST /repos/laudebugs/lamina/actions/runners/generate-jitconfig` (name, the three labels, default runner group) and passes the result to the container in an environment variable, never on a command line or in a file; `entrypoint.sh` drops it from the environment before the runner starts. The config holds the credentials of that one ephemeral runner, which GitHub deletes after its job. Nothing secret is in the repository, the image or the volumes.

The price: the supervisor is a host process (a launchd agent) rather than Docker's restart policy, and it uses the owner's `gh` login, so logging `gh` out stops new registrations (the log says so; `gh auth login` and the next retry picks it up).

## Resources and limits

- `--cpus 3 --memory 6g`, so the macOS runners and local work keep most of the 8-core, 16 GB Mac. If the macOS jobs slow down by more than about 20%, lower the cap in `loop.sh` (and rerun `up.sh`) or take one macOS runner offline while this one runs.
- Named volumes `lamina-linux-rustup` (`~/.rustup`) and `lamina-linux-cargo` (`~/.cargo`, including the registry and the pinned `cargo-nextest`/`cargo-deny`) persist between jobs, so Rust installs once. Everything else, including `target/`, starts empty: every job is a cold build.
- One job at a time. ARM64 only: x86_64-specific code paths aren't covered (todo X5's risks).
- Self-hosted runners must never run fork PRs; that holds while the repository is private.

## Updating the runner

Dependabot (`docker` ecosystem in `.github/dependabot.yml`) bumps the pinned base image weekly; after it merges, rerun `up.sh` on the Mac. GitHub stops serving runners more than about a month behind the current release, and a runner that's behind updates itself at the start of each job (slower, but it works) until the image catches up.
