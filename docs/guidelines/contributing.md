# Contributing

Start here. This page is the checklist. The linked guides have the rules and the reasons behind them.

## Before you start

1. Read [`CLAUDE.md`](../../CLAUDE.md) for the working agreements and commands.
2. For PDF features, read the relevant chapter in [`docs/research/`](../research/) and the ISO 32000 section first (*spec first*).
3. Check [`docs/decisions/`](../decisions/) for ADRs that constrain the change.

## The workflow in brief

1. **Create a new git worktree off up-to-date `main`** with a branch named `<type>/<short-description>`: `git worktree add -b feat/xref-repair feat-xref-repair main`. See [git-workflow.md](git-workflow.md).
2. Make one focused change, following [code-organization.md](code-organization.md), [maintainability.md](maintainability.md) and the language guide ([rust.md](rust.md), [typescript.md](typescript.md)). Docs and examples readers see also follow [docs-writing.md](docs-writing.md) and [examples.md](examples.md).
3. Commit with [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): summary`. Types: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`, `spike`.
4. Push and open a pull request against `main`, with a summary, sources and a test plan. Don't push directly to `main`.
5. Get a review, address every comment, and merge once checks pass.
6. Remove the worktree and branch.

## Changesets

Lamina PDF has one version for every crate and every npm package: `@laminapdf/wasm`, `@laminapdf/sdk`, `@laminapdf/render`, `@laminapdf/render-hayro`, `@laminapdf/render-pdfium` and `@laminapdf/viewer` ([ADR 0017](../decisions/0017-versioning-and-release.md)). A PR that changes what users of the library see (an API, behaviour, output bytes, a fix they'd notice) adds a **changeset**: run `pnpm changeset`, pick the packages the change touches (they are one fixed group, so any of them bumps all six, and the root `CHANGELOG.md` merges every package's entries, so naming only a render package is enough), choose the bump and write one or two sentences for the changelog. It writes `.changeset/<name>.md`; commit it with the change. Docs, CI, tests and internal refactors need none.

- **Bump while the version is `0.x`:** `minor` for a breaking change (see the [stability policy](../stability-policy.md) for what counts), `patch` for everything else. Never `major`.
- **Never edit a version by hand.** `cargo xtask check` runs `cargo xtask version sync --check`, which fails when a crate, `Cargo.lock`, `CHANGELOG.md` or `@laminapdf/sdk`'s `VERSION` (`packages/sdk/src/version.ts`) disagrees with the packages' version, or when the fixed group's packages disagree, or when a crate sets its own version instead of `version.workspace = true`.
- **Releases** run after a full nightly passes on `main`, when changesets are pending. The daily train prepares a version commit on the tested commit and pushes its branch and tag with the release bot. The release lane checks and publishes it, deploys its docs, then opens a version PR with auto-merge enabled. Required checks gate the merge back to `main`. [Daily releases](../../tools/release/README.md) covers setup, preparation without pushing, the `RELEASE_TRAIN=off` switch and recovery.
- **Manual releases** remain available to the owner: `pnpm changeset version`, then `cargo xtask version sync`, committed in one PR. After it merges, tag `v<version>` on `main`. Only a tag push publishes. `gh workflow run release.yml --ref main` runs the lane without publishing; a dispatch on a tag is also a dry run.
- While the version is a pre-release (`-alpha.N`), Changesets is in pre mode (`.changeset/pre.json`): consumed changesets move to `.changeset/pre/`. The alphas end at `0.2.0` (ADR 0017, amended 2026-11-11): from then on a version has no pre-release part, and a breaking change takes a `minor` changeset.

## Documentation

- Anything worth remembering goes in `docs/`. See [`docs/README.md`](../README.md) for where.
- Significant technical decisions get an ADR in `docs/decisions/`.
- Add every new doc to its folder's `README.md` index.
- Cite primary sources, and label recommendations and opinions as such.

## Dependencies and licences

Check the licence of every new dependency, fixture PDF and font before adding it. No AGPL/GPL/LGPL code goes into shipped artefacts. See [research ch. 05 §8.4](../research/pdf/05-creating-pdfs-from-code.md#84-licensing-traps-to-avoid).

## Lock the actions

The workflows name each action by its release tag (`actions/checkout@v7.0.1`), and `.github/workflows/actions.lock` records the commit that tag resolved to and the repository that owns it ([ADR 0019](../decisions/0019-supply-chain-and-licence-hygiene.md)). GitHub refuses to run an action the lockfile doesn't cover, so after adding a workflow or changing a `uses:` line:

1. Install the extension at the pinned version: `cargo xtask doctor --install` (or the `gh extension install` command `cargo xtask doctor` prints).
2. Run `gh actions-lock` at the repository root and commit `.github/workflows/actions.lock` with the workflow change.
3. Write this repository's own actions as `$/.github/actions/<name>`, never `./`: the lockfile doesn't follow `./` references, and a test fails on one.

The fast lane's `lockfile` job runs `gh actions-lock --verify` on every pull request. Dependabot's update of an action a workflow names changes the tag and its lockfile entry together; if its update of an action a composite action names fails the job, run `gh actions-lock` on its branch.

When the job reports that a locked commit has moved, an upstream maintainer retagged a release. Find out why before relocking: compare the old and new commits in the action's repository. `gh actions-lock --relock` and `--accept-moved` take the new commit; use either only in a reviewed PR whose description says why, never in automation.
