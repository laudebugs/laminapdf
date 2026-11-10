# Daily releases

A full nightly on `main` starts the daily release. The train checks out the commit that the nightly tested, asks Changesets for its next version, and prepares a version commit. With no pending changeset, it ends without pushing anything.

The release bot creates `release/v<version>` and `v<version>`. It uploads the prepared blobs and tree, then asks GitHub's Commits API to create a verified commit as the GitHub App, without custom author, committer or signature fields, as required by [GitHub's bot-signature rules](https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification#signature-verification-for-bots). The train checks GitHub's signature result and the exact tree and parent before creating the branch and tag. The tag starts the existing release lane: checks, publishing, registry installs, the GitHub release and production docs. Once those succeed, the bot opens the version PR and enables squash auto-merge. Required PR checks still gate that merge.

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

### The web reader's train

The web reader releases on its own, apart from the packages ([C7e](../../docs/todos/done/C7e-automated-web-releases.md)). `web-release-train.yml` runs after the same full nightly. When what the reader ships changed since its newest `web-v*` tag, it tags `web-v<next>` on the nightly's commit with the release bot, and that tag runs `web-deploy.yml`'s release gates and production deploy. `cargo xtask web next-release` decides. The patch number goes up by itself. For a new minor or major, raise `apps/web/package.json`'s version in a PR, and the next release takes it as written. Install it the same way:

```sh
cp tools/release/web-release-train.yml .github/workflows/web-release-train.yml
git add .github/workflows/web-release-train.yml
git commit -m "build: install the web reader's release train"
git push origin HEAD
```

A release counts as made once its deploy passes the smoke test and creates its GitHub release. When a deploy fails, the next night's release, under a new tag, includes that tag's changes again. Create `web-v0.1.0`'s GitHub release by hand once, from [its notes](../../apps/web/CHANGELOG.md) (`gh release create web-v0.1.0 --verify-tag --notes-file apps/web/CHANGELOG.md`): it predates the train, and the train counts changes from the newest release. Set the repository variable `WEB_RELEASE_TRAIN=off` to stop it. To release by hand, run the workflow with a nightly's run ID, or push a `web-v<version>` tag on a commit of `main`. Each release gets a GitHub release with the tag's notes once its deploy passes the smoke test.

The GitHub App needs contents and pull-request write access to this repository. Set its ID and private key as repository secrets `RELEASE_APP_ID` and `RELEASE_APP_PRIVATE_KEY`. Enable auto-merge, squash merging and automatic deletion of merged branches in the repository settings. These settings and the bot secrets were configured by the owner on 2026-11-05.

The existing publishing setup must also be ready: crates.io and npm trusted publishers for `release.yml`, and the `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` secrets for production docs. A missing docs credential fails the release; it cannot silently skip deployment and merge the version PR.

## Run the train by hand

In GitHub Actions, open **Daily release**, choose **Run workflow**, select `main`, and enter the ID of a completed, successful full nightly on `main`. A scheduled nightly qualifies, as does a manual nightly with every budget at its default.

The CLI equivalent below uses `NIGHTLY_RUN_ID` as a placeholder for that run's numeric ID:

```sh
gh workflow run release-train.yml --ref main -f nightly-run-id=NIGHTLY_RUN_ID
```

The train validates the run before checking out its commit. It then follows the automatic path, including the kill switch, duplicate-release refusals and release-change check. Before pushing, it checks that the latest full nightly still permits this release; an older selection may be refused. Nothing is pushed when no changesets are pending.

Manual execution uses the selected nightly's commit, so it cannot pick up a preparation fix added after that nightly. Run a full nightly on a commit containing the fix first.

## Prepare without pushing

From a clean checkout of the exact commit of a completed full nightly on `main`, install the pinned tools and workspace dependencies, then run:

```sh
cargo xtask release prepare --run <nightly-run-id>
```

The command validates the run again. When a detached checkout lacks the configured Changesets base ref, it creates that branch at the tested commit. It preserves existing refs and keeps HEAD on the tested commit.

It asks `pnpm changeset status` for the next version, refuses an open version PR, any remaining `release/v*` branch or an existing tag, then runs `pnpm changeset version`, `version sync` and the package-docs generator. It force-adds the otherwise ignored crate-docs snapshot, checks that the tree contains only the version bump and generated package docs, and commits it on `release/v<version>`. The version PR brings that snapshot to `main`, where it stays until the next release replaces it. `cargo xtask publish crates` regenerates the docs either way: on a release commit the output matches the snapshot and Cargo publishes a clean tree; on any other commit (a dry run on `main`, or a release tagged by hand) it publishes the generated docs with `--allow-dirty`, and refuses any other uncommitted change. It never tags, pushes, publishes or opens a PR.

In Actions it writes `ready=false` when nothing is pending, or `version`, `branch`, `tag` and `ready=true` after the commit succeeds. Use a disposable checkout when trying it by hand: a failed preparation leaves its files and branch for inspection.

## Stop or recover a release

Set the repository variable `RELEASE_TRAIN` to `off` to stop future trains. It does not cancel a release already publishing. The owner can still push a release tag by hand; a dispatched release lane remains a dry run.

A failed release leaves its branch and, once created, its tag in place. A signing or verification failure can leave only the prepared branch; no tag means the release lane did not start. The next train refuses to replace either ref. Fix the failing check or publishing setup and rerun the failed release jobs; publishing skips versions already on a registry. If the GitHub-release job alone failed after creating its release, inspect that release before rerunning it: its existing create step is not idempotent.

If the version PR conflicts with later merges, merge `main` into its branch and finish the PR by hand after checks pass. The release lane refuses to enable auto-merge when that branch differs from the published commit. Automatic branch deletion removes the blocker after the PR merges. A closed, unmerged release PR requires the owner's recovery before another release.

After installation and merge, check the first full nightly's daily-release run. Confirm the same version is on both registries and the production docs, and that its version PR merges. This is the remaining post-merge check in the [task](../../docs/todos/done/P6-daily-release.md).

## The CLI's release

The `laminapdf` command-line tool releases apart from the libraries, from a `laminapdf-cli-v<version>` tag. [`.github/workflows/cli-release.yml`](../../.github/workflows/cli-release.yml) builds and installs all eight targets, then publishes three things: the GitHub release, the nine npm packages and the Homebrew formula. It publishes only on that tag's push, while the repository variable `LAMINA_PDF_PUBLISH_CLI` is `true`.

Before the first release, the owner does this once:

1. **Create the tap.** Make a public repository `laudebugs/homebrew-tap`. Homebrew installs `brew install laudebugs/tap/laminapdf` from its `Formula/laminapdf.rb`, which the release writes.
2. **Let the release bot write to it.** Install the release bot, the GitHub App behind `RELEASE_APP_ID` and `RELEASE_APP_PRIVATE_KEY`, on `laudebugs/homebrew-tap` with read and write access to its contents. The release asks for a token for that repository alone.
3. **Publish each npm package once.** npm sets a trusted publisher only on a package that already exists. Run the workflow by hand (`gh workflow run cli-release.yml --ref main`) and download its `cli-npm` artifact. Its `publish.json` lists the nine tarballs in order. Run `npm login`, then publish each with `npm publish <tarball> --access public --tag <tag from publish.json>`, in that order.
4. **Add the trusted publisher.** On npmjs.com, open each of the nine packages' settings and add a trusted publisher: GitHub Actions, repository `laudebugs/laminapdf`, workflow `cli-release.yml`, environment `release`.
5. **Turn publishing on.** Set the repository variable `LAMINA_PDF_PUBLISH_CLI` to `true`.

Then tag a release from `main`: `git tag laminapdf-cli-v<version>` (the workspace's version) and push the tag. A failed publish job can be re-run: npm skips versions it has, and the tap's formula is replaced. Until the repository is public, the formula's archives are on a GitHub release only collaborators can download, so `brew install` works only for them.
