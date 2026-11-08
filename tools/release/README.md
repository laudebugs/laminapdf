# Daily releases

A full nightly on `main` starts the daily release. The train checks out the commit that the nightly tested, asks Changesets for its next version, and prepares a version commit. With no pending changeset, it ends without pushing anything.

The release bot pushes `release/v<version>` and `v<version>` together. The tag starts the existing release lane: checks, publishing, registry installs, the GitHub release and production docs. Once those succeed, the bot opens the version PR and enables squash auto-merge. Required PR checks still gate that merge.

## Install the workflows

The agent's GitHub access cannot push workflow files. On this PR's branch, the owner copies the reviewed files into place and commits them:

```sh
cp tools/release/release-train.yml .github/workflows/release-train.yml
cp tools/release/release.yml .github/workflows/release.yml
git add .github/workflows/release-train.yml .github/workflows/release.yml
git commit -m "build: install the daily release workflows"
git push origin HEAD
```

`cargo test -p xtask` compares the installed workflows with these files. It fails until both are installed. Keep these copies in sync when changing either workflow.

The GitHub App needs contents and pull-request write access to this repository. Set its ID and private key as repository secrets `RELEASE_APP_ID` and `RELEASE_APP_PRIVATE_KEY`. Enable auto-merge, squash merging and automatic deletion of merged branches in the repository settings. These settings and the bot secrets were configured by the owner on 2026-11-05.

The existing publishing setup must also be ready: crates.io and npm trusted publishers for `release.yml`, and the `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` secrets for production docs. A missing docs credential fails the release; it cannot silently skip deployment and merge the version PR.

## Run the train by hand

In GitHub Actions, open **Daily release**, choose **Run workflow**, select `main`, and enter the ID of a completed, successful full nightly on `main`. A scheduled nightly qualifies, as does a manual nightly with every budget at its default.

The CLI equivalent below uses `NIGHTLY_RUN_ID` as a placeholder for that run's numeric ID:

```sh
gh workflow run release-train.yml --ref main -f nightly-run-id=NIGHTLY_RUN_ID
```

The train validates the run before checking out its commit. It then follows the automatic path, including the kill switch, duplicate-release refusals and version-only check. Before pushing, it checks that the latest full nightly still permits this release; an older selection may be refused. Nothing is pushed when no changesets are pending.

Manual execution uses the selected nightly's commit, so it cannot pick up a preparation fix added after that nightly. Run a full nightly on a commit containing the fix first.

## Prepare without pushing

From a clean checkout of the exact commit of a completed full nightly on `main`, install the pinned tools and workspace dependencies, then run:

```sh
cargo xtask release prepare --run <nightly-run-id>
```

The command validates the run again. When a detached checkout lacks the configured Changesets base ref, it creates that branch at the tested commit. It preserves existing refs and keeps HEAD on the tested commit.

It asks `pnpm changeset status` for the next version, refuses an open version PR, any remaining `release/v*` branch or an existing tag, then runs `pnpm changeset version` and `version sync`. It stages the result, checks the version-only rule and commits it on `release/v<version>`. It never tags, pushes, publishes or opens a PR.

In Actions it writes `ready=false` when nothing is pending, or `version`, `branch`, `tag` and `ready=true` after the commit succeeds. Use a disposable checkout when trying it by hand: a failed preparation leaves its files and branch for inspection.

## Stop or recover a release

Set the repository variable `RELEASE_TRAIN` to `off` to stop future trains. It does not cancel a release already publishing. The owner can still push a release tag by hand; a dispatched release lane remains a dry run.

A failed release leaves its branch and tag in place. The next train refuses to replace them. Fix the failing check or publishing setup and rerun the failed release jobs; publishing skips versions already on a registry. If the GitHub-release job alone failed after creating its release, inspect that release before rerunning it: its existing create step is not idempotent.

If the version PR conflicts with later merges, merge `main` into its branch and finish the PR by hand after checks pass. The release lane refuses to enable auto-merge when that branch differs from the published commit. Automatic branch deletion removes the blocker after the PR merges. A closed, unmerged release PR requires the owner's recovery before another release.

After installation and merge, check the first full nightly's daily-release run. Confirm the same version is on both registries and the production docs, and that its version PR merges. This is the remaining post-merge check in the [task](../../docs/todos/done/P6-daily-release.md).
