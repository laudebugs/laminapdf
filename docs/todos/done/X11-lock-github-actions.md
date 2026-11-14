# X11 — Lock the GitHub Actions the workflows use

| Field | Value |
| --- | --- |
| ID | `X11` |
| Status | `done` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | None |
| ADRs | Amends [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) ("GitHub Actions are pinned by commit SHA"); follows [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) for the new fast-lane job |
| Branch | `ci/x11-lock-github-actions` |
| PR title | `ci: lock the workflows' actions with a lockfile, and check it on every pull request` |
| Size | M (the rewritten `uses:` lines and the generated lockfile don't count) |
| PR | [laudebugs/laminapdf#580](https://github.com/laudebugs/laminapdf/pull/580) |

## Goal

Every action the workflows use is pinned to a full commit SHA with its version in a comment, as ADR 0019 asks. SHA pins stop a moved tag from changing the code we run, but they are hard to read and review. They also stop at the first level: an action we pin that itself calls another action by tag runs whatever that tag points at today. GitHub's workflow lockfiles became generally available on 2026-10-28. A lockfile, `.github/workflows/actions.lock`, records the commit each `uses:` reference resolves to, transitive actions included, and the repository behind it. GitHub then runs the locked commit, and refuses to run an action whose commit or repository doesn't match. This todo locks every workflow, checks the lockfile on every pull request, and turns on the repository's **Require lockfile** policy.

## Context

- [Hugo van Kemenade: Lock your GitHub Actions](https://hugovk.dev/blog/2026/lock-github-actions/): one maintainer's adoption across several Python projects. Read it for the steps, the verify job, and the Dependabot and zizmor notes.
- GitHub's [announcement](https://github.blog/changelog/2026-10-28-github-actions-workflow-lockfiles-are-generally-available/) and docs:
  - [workflow dependency lockfiles](https://docs.github.com/actions/concepts/security/workflow-dependency-lockfiles): what the lockfile records and enforces, and what it leaves out;
  - [locking a repository's workflows](https://docs.github.com/en/actions/how-tos/secure-your-work/lock-workflow-dependencies): the `gh actions-lock` commands and local actions;
  - [enforcing lockfiles](https://docs.github.com/en/actions/how-tos/administer/enforce-workflow-lockfiles): the verify job and the **Require lockfile** policy.
- [`github/gh-actions-lock`](https://github.com/github/gh-actions-lock): the CLI extension, its `verify-lockfile.yml` and its [troubleshooting guide](https://github.com/github/gh-actions-lock/blob/main/docs/troubleshooting.md).
- [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md): the rule this todo replaces, and the Dependabot setup it relies on.
- `.github/dependabot.yml`'s `github-actions` entry: weekly, a 7-day cooldown, and the directories `/` and `/.github/actions/*`.
- `.github/actions/*/action.yml`: 13 local composite actions, which most jobs call as `./.github/actions/<name>`. Three of them call actions from other repositories, and two call other local actions.
- `tools/xtask/src/workflows.rs` and `workflows/compile_cache.rs`: the tests that read the workflows, 14 of them by the `./.github/actions/` path.
- `tools/xtask/src/doctor.rs`: where external tool versions are pinned; CI installs the same ones.

## What the lockfile does and doesn't cover

These facts, from GitHub's docs as of 2026-11-13, shape the tasks. Check them against the docs and `gh actions-lock --help` when the work starts.

- **Covered:** every `owner/repo@ref` action in an onboarded workflow and the actions those actions use. GitHub refuses a run when an action isn't in the lockfile, when the locked commit isn't reachable from its tag in the action's own repository, or when the repository behind the name has changed (`owner_id`, `repo_id`).
- **Local actions written `./…` are not covered.** `gh actions-lock` doesn't follow them. When one calls an action from another repository, it stops with a `local-action` error and writes nothing. It offers two ways out:
  - `--migrate-local-actions` rewrites `./.github/actions/<name>` as `$/.github/actions/<name>`, a reference to this repository's own action at the workflow's commit, whose actions are then locked.
  - `--ignore-local-actions` leaves `./` and needs a never-run composite action per workflow that lists the actions the local ones use.
- **Reusable workflows are not covered.** `release.yml` calls `./.github/workflows/nightly.yml` and `docs.yml`, which are in this repository and run at the same commit, so nothing changes for them.
- **Dependabot updates locked entries but onboards nothing.** A workflow added later needs `gh actions-lock` run by hand, and the verify job fails until it is.

## Scope

**In scope**

- The extension pinned in `doctor.rs` and installed by `cargo xtask doctor --install`, so contributors and CI run one version.
- Every workflow onboarded with `gh actions-lock`. The `uses:` lines go back to version tags (`actions/checkout@v7.0.1`), and the lockfile holds the SHAs.
- The local composite actions moved to `$/` references (`--migrate-local-actions`), so the actions they call are locked too. Use `--ignore-local-actions` instead only if `$/` turns out to break something (see Risks).
- A `lockfile` job in the fast lane (`ci.yml`) that runs `gh actions-lock --verify`, is required by `fast-gate`, and runs on every pull request, docs-only ones included.
- The workflow tests: the verify job, the extension pin matching `doctor.rs`, and the 14 `./.github/actions/` matches moved to the new spelling.
- ADR 0019 amended: actions are locked by the lockfile, not pinned by SHA in the workflow files.

**Out of scope** (and which todo covers it)

- zizmor, the Actions security linter the post also recommends (v1.32.0 is the first to accept lockfiles instead of SHA pins). It has no todo yet; it would be its own.
- Renovate: ADR 0019 keeps Dependabot.
- Container images (`tools/ci-runner/linux`) and the actions in `spikes/`, which no workflow runs.

## Found while building it

Checked against `gh-actions-lock` v0.4.1's README and troubleshooting guide on 2026-11-13:

- **The extension keeps SHA pins.** It locks a full-SHA `uses:` line unchanged, with a `not-pinned` warning, rather than turning it back into a tag. So the PR rewrites every `@<sha> # vX.Y.Z` to `@vX.Y.Z` itself. Each pinned SHA was first checked against its tag in the action's own repository with `git ls-remote`, and all matched.
- **Local actions were rewritten by hand first.** Every `./.github/actions/<name>` in the workflows and composite actions is written `$/.github/actions/<name>`, so `--migrate-local-actions` found nothing to change. Calls of reusable workflows (`./.github/workflows/nightly.yml`, `docs.yml`) stay `./`, as the docs say.
- **Container images:** the extension warns on an image without an `@sha256:` digest. No workflow uses a container.
- **Two sessions finished it.** The first couldn't push to `.github/workflows/` (its token had no `workflow` scope) or reach GitHub's API for the extension, so it pushed everything but the workflows. The second session had a token with the `workflow` scope. It rechecked the 15 SHA and tag pairs with `git ls-remote` (all matched), rewrote the 14 workflows, added the `lockfile` job and ran `gh actions-lock --migrate-local-actions` at v0.4.1. That run wrote only `.github/workflows/actions.lock`, rewrote nothing and warned about nothing.
- **The lockfile follows the composite actions.** Through the `$/` references it locks what they call (`pnpm/action-setup`, `nick-fields/retry`, `taiki-e/install-action`, `actions/setup-node`, `actions/cache`), and two transitive actions: `Swatinem/rust-cache` through `actions-rust-lang/setup-rust-toolchain`, and `actions/attest` through `actions/attest-build-provenance`. `cancel-merged.yml` uses no action and is onboarded with an empty list.
- **Checked locally:** `gh actions-lock --verify` passes on all 14 workflows, and fails (exit 1) when one `uses:` tag changes without relocking.

## Owner's steps

After merge, do the first two post-merge checks below: turn on **Require lockfile** once the first push to `main` passes with the lockfile, then check the allowed-actions list.

## Tasks

- [x] Read `gh actions-lock --help` and the troubleshooting guide at the version you pin. Note in the PR anything that differs from "What the lockfile does and doesn't cover" above.
- [x] Pin the extension in `tools/xtask/src/doctor.rs` (`GH_ACTIONS_LOCK_VERSION`), with `doctor` checking `gh extension list` and `doctor --install` running `gh extension install github/gh-actions-lock --pin <version>`.
- [x] Run `gh actions-lock --migrate-local-actions`. Review every rewritten line: each `./.github/actions/<name>` should now read `$/.github/actions/<name>`, and each pinned SHA its tag.
- [ ] Run a branch push and check that one job of each kind still finds its local action at the right commit: a fast-lane Rust leg and the full lane's `check-windows`. The nightly and release dispatches need `main`: see Post-merge checks.
- [x] Commit the rewritten workflows, the composite actions and `.github/workflows/actions.lock`.
- [x] Add a `lockfile` job to `ci.yml`, modelled on the extension's `verify-lockfile.yml`:
  - on `ubuntu-24.04`, as the repository's other Linux jobs, with `permissions: contents: read`;
  - `actions/checkout` with `persist-credentials: false`;
  - the extension installed at the `doctor.rs` pin;
  - `gh actions-lock --verify`, with `GH_TOKEN: ${{ github.token }}`.
  Add it to `fast-gate`'s `needs`, and give it its own line in the gate's `case` (`lockfile) needed=true ;;`). Without that line the gate reads it under the `ts` flag, so on a PR where `ts` is false the job's `success` fails the gate. Keep it out of `cargo xtask changes`' skips: a workflow edit is the one change it exists to catch.
- [x] Update `tools/xtask/src/workflows.rs` and `workflows/compile_cache.rs`: their path matches, plus new tests that the `lockfile` job exists, runs `--verify`, is in `fast-gate`'s `needs` with `needed=true` and installs the `doctor.rs` pin.
- [x] Check that Dependabot still updates the actions the composite actions call. Its `/.github/actions/*` directories stay. Whether its update of a composite action's `uses:` line also rewrites the lockfile is unconfirmed until its first weekly PR (a post-merge check); if not, the `lockfile` job fails and `contributing.md` says to run `gh actions-lock` on its branch.
- [x] Amend ADR 0019: actions are locked by `.github/workflows/actions.lock`, with tags in the workflow files; `gh actions-lock` after adding a workflow or changing a `uses:` line; the `lockfile` job. Leave the date of the change and a link to this todo.
- [x] Add a "Lock the actions" note for contributors to `docs/guidelines/contributing.md`: when to run `gh actions-lock`, and that `--relock` and `--accept-moved` need a reviewed reason.

## Acceptance criteria

- [x] `.github/workflows/actions.lock` exists and `gh actions-lock --verify` passes on the PR's head.
- [x] Every workflow under `.github/workflows/` is onboarded; no `uses:` line names a bare SHA, and no local action is written `./`.
- [ ] The `lockfile` job runs on the PR and is required by `fast-gate`, which passes on a docs-only PR too.
- [x] A test branch that changes one `uses:` tag without relocking fails the `lockfile` job. (Checked locally: `gh actions-lock --verify` exits 1.)
- [x] The workflow tests pass, including the new ones.

## Post-merge checks

- [ ] The owner turns on **Require lockfile** for the repository (the page GitHub's enforcement guide names) in Active mode, after the first push to `main` passes with the lockfile. Evaluate mode needs GitHub Enterprise Cloud, so this repository can't use it.
- [ ] The owner checks the repository's allowed-actions list, if it has one: after onboarding, GitHub checks allowlists against the locked SHAs, so an entry written with a tag (`actions/checkout@v6`) stops matching.
- [ ] The next weekly Dependabot `github-actions` PR updates both the `uses:` tag and its lockfile entry, and passes the `lockfile` job.
- [ ] The nightly, the release lane and every deploy workflow run once on the lockfile without a refused action.

## Tests

`tools/xtask/src/workflows.rs` (the `lockfile` job, its pin, `fast-gate`'s `needs`; the composite action paths in their new spelling) and `workflows/compile_cache.rs`. The `lockfile` job itself checks the lockfile in CI.

## Docs to update

ADR 0019 (the amendment), the architecture plan's §7 Cross-cutting (tick X11), `docs/guidelines/contributing.md`, `.github/dependabot.yml`'s comment if what it covers changes, and CLAUDE.md's `.github/` line.

## Risks and open questions

- **An agent can't push this PR.** GitHub refuses workflow changes from tokens without the `workflow` scope, and agents' tokens usually lack it ([#575](https://github.com/laudebugs/laminapdf/pull/575) ran into this). The owner pushes the workflow commits, or gives the session a token that can.
- **`$/` references are new.** Before moving 13 composite actions and every job that calls them, try `$/` on one job of each kind (the task above). Check too whether a `$/` step still needs `actions/checkout` before it, as a `./` one does; GitHub's docs don't say. If anything misbehaves, use `--ignore-local-actions` with one never-run composite action per workflow that lists the actions the local ones use. That costs a file per workflow, kept in step by hand, so prefer `$/`.
- **A moved tag fails every run.** GitHub refuses a locked commit that its tag no longer reaches, and `--relock` or `--accept-moved` is needed. That is the point, but it can stop CI on a day when an upstream maintainer retags a release. The contributing note says how to check and relock.
- **The extension is pre-1.0.** The lockfile format is `v0.0.3` and the extension `v0.4.x` as of the post. Pin it and bump it in a reviewed PR, as for the other tools in `doctor.rs`.
- **Dependabot and composite actions.** GitHub's docs say Dependabot regenerates entries for onboarded workflows. Whether that includes actions reached through a `$/` composite action is unconfirmed; the Dependabot task above finds out.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
