# Plan: a daily automatic release, and docs that match it

- **Status:** Draft
- **Date:** 2026-11-05
- **Asked for by:** the project owner (2026-11-05): "versioning for docs, so that each docs deployment has a matching version", "publish the packages automatically each time there's a change, so that users can match docs versions with the package versions, instead of making package deployments manual". Decided the same day: a daily release after the nightly passes; npm publishing without the owner's approval step, from a GitHub-hosted runner; a GitHub App for the release bot, which the owner creates; unreleased docs at `next.docs.laminapdf.com`.
- **Changes:** [ADR 0017 §5–§7](../decisions/0017-versioning-and-release.md#5-tags) (who tags, the release lane's nightly run, npm publishing, the docs), [ADR 0011 §1](../decisions/0011-cloudflare-topology-and-privacy.md#1-topology) (the docs hosts) and [ADR 0018 §2](../decisions/0018-test-strategy.md#2-ci-lanes) (the Release row)
- **Builds on:** the release lane and its publish jobs ([P1](../todos/done/P1-reserve-names-and-trusted-publishing.md), [P2](../todos/done/P2-first-registry-prerelease.md)), the registry checks ([P3](../todos/ready/P3-registry-checks.md)) and the docs site's deploys (`.github/workflows/docs.yml`)
- **Todos:** [§6](#6-todos)

## 1. Goal

Lamina PDF has published two alphas, `0.2.0-alpha.1` and `0.2.0-alpha.2`, and each one needed the owner twice: to push the tag, then to approve every npm package on npmjs.com. Between releases the docs site deploys every merge to `main`, so it describes code that no registry has yet, and a reader can't tell which version a page is about. This plan makes a release happen on its own once a day, when the nightly lane has passed and merged changes are waiting. It also ties the docs to that release: `docs.laminapdf.com` always documents the version on crates.io and npm, and `next.docs.laminapdf.com` documents what `main` holds beyond it.

## 2. What a release needs today, and what changes

The release lane ([ADR 0017 §6](../decisions/0017-versioning-and-release.md#6-the-release-lane)) stays the gate. It refuses a commit unless a full nightly passed on it or an ancestor, runs the fast lane, the full lane's suites and the nightly lane on it, installs the packed packages in every runtime, and only then publishes. Four parts of it change:

- **Who tags.** The owner pushes `v<version>` today. A release bot, a GitHub App the owner creates, pushes it instead (§3). A tag pushed with the workflow's own token would start no other workflow, so the bot needs its own token.
- **The lane's own nightly run.** Running the nightly lane again on the release commit takes hours on the shared Mac. When the release commit differs from a commit the nightly already passed on only in the files a version bump writes, the lane skips that run.
- **npm.** Staged publishing with a stage-only token, approved by the owner, gives way to npm's trusted publishing. npm accepts trusted publishing only from GitHub-hosted runners ([npm's docs](https://docs.npmjs.com/trusted-publishers)), so one short job runs there. It works from a private repository, though without provenance until the repository is public.
- **The docs.** Production docs deploy from each release instead of from every push to `main` (§4).

## 3. The daily release

A new workflow, `release-train.yml`, runs when the nightly lane completes:

1. **Only after a full nightly passes on `main`.** A scheduled run, or a dispatched one with every input at its default, the same runs the release guard counts. Any other run, a failure or a cancelled run stops it.
2. **Only when something waits.** `cargo xtask release prepare` reads the pending changesets on the nightly's commit. With none, there is nothing to release and the workflow ends. It also refuses while a version pull request is open or the next version's tag exists, so a stuck release is never cut twice.
3. **The version commit.** On a branch `release/v<version>` from the nightly's commit, it runs `pnpm changeset version` and `cargo xtask version sync`, checks that only version files changed, and commits `release: v<version>`.
4. **The tag.** The bot pushes the branch and the tag `v<version>` on that commit, which starts the release lane. The lane's guard finds the nightly on the commit's parent, and its version-only check skips the second nightly run.
5. **Publishing.** The lane publishes the crates (trusted publishing, as today) and the npm packages (trusted publishing from a GitHub-hosted runner), writes the GitHub release, and deploys the docs for the version.
6. **Back to `main`.** The bot opens the version pull request from `release/v<version>` and, once the lane has published, turns on its auto-merge. The pull request runs the usual checks and merges like any other, which removes the consumed changesets and bumps `main`'s version.

The tag sits on the version commit, and the squash of the version pull request puts a copy of its change on `main`, so the tagged commit itself never lands on `main`: the tag is what keeps it once the branch is deleted. Its tree is the nightly's tree plus the version bump, which is exactly what was tested and published. Nothing reads tags from `main`'s history: the release guard compares the nightly's commit with the tagged one, and `cargo xtask progress` reads pull request titles.

The cadence is at most one release a day, set by the nightly. A release on every merge would leave the pull request's checks as the only gate, since the lane takes hours and the project merges about ten pull requests a day. The repository variable `RELEASE_TRAIN` set to `off` stops the daily release, and the owner can still tag a release by hand.

## 4. Docs that match a version

- **Every page names its version.** The docs site's header shows the version it documents, read from the fixed group's version at build time, and the install lines name it exactly (`npm install @laminapdf/sdk@0.2.0-alpha.3`, `cargo add laminapdf@0.2.0-alpha.3`) beside the `alpha` tag.
- **`docs.laminapdf.com` is the latest release.** The release lane deploys the docs from the release commit once the packages are published, so the live docs never run ahead of the registries.
- **`next.docs.laminapdf.com` is `main`.** Every push to `main` deploys the docs to a second Worker, `lamina-pdf-docs-next`. Its pages say they describe unreleased changes and link to the released docs, and the build marks them `noindex`, as previews are today.
- **Older versions are not kept** while the version is an alpha. A version switcher, with each minor version's docs kept, can come with `1.0`.
- **Previews per pull request** stay as they are.

## 5. Risks

- **The bot's key is a repository secret.** Any workflow on a same-repository pull request can read repository secrets, and environments that would fence them off aren't available to a private repository on the owner's plan. The app gets only contents and pull-request permissions on this one repository, and the owner can revoke it in one step.
- **A version pull request can conflict.** Pull requests merged after the nightly's commit may touch the same lines, most likely in `Cargo.lock`. The release is published by then, so the fix is a merge of `main` into the branch. The next day's release waits until the pull request has merged.
- **A second-level name needs its own certificate.** Cloudflare's universal certificate covers `*.laminapdf.com` but not `*.docs.laminapdf.com`. P4 checks first that the Workers custom domain issues one for `next.docs.laminapdf.com`, and asks the owner otherwise.
- **GitHub-hosted minutes.** The npm job is the project's first on a GitHub-hosted runner. It downloads the tarballs and publishes them, a few minutes a release, and runs no code from the repository.
- **Docs between a merge and its release.** A change merged today is on `next.docs` at once but reaches `docs.laminapdf.com` only with tomorrow's release. That is the point: the live docs match what users can install.

## 6. Todos

| ID | Todo | Depends on |
| --- | --- | --- |
| [P4](../todos/ready/P4-docs-match-the-release.md) | Docs that match the release: the version on every page, exact install lines, `main` at `next.docs.laminapdf.com`, production docs from the release lane | P3 |
| [P5](../todos/ongoing/P5-npm-trusted-publishing-and-version-only-releases.md) | npm trusted publishing from a GitHub-hosted runner, and a release lane that skips its nightly run for a version-only commit | P2 |
| [P6](../todos/ready/P6-daily-release.md) | The daily release: `release-train.yml`, `cargo xtask release prepare`, the release bot and its version pull request | P4, P5 |

Workflow files can't be pushed by the agents' GitHub access, so each todo's pull request carries its workflow changes for the owner to commit.

## 7. To-dos

- [ ] Show the released version on every docs page, and serve `main`'s docs at `next.docs.laminapdf.com` ([P4](../todos/ready/P4-docs-match-the-release.md))
- [ ] Publish to npm without a stored token or an approval step, and skip the release lane's second nightly run for a version-only commit ([P5](../todos/ongoing/P5-npm-trusted-publishing-and-version-only-releases.md))
- [ ] Release once a day, after the nightly passes, with no human step ([P6](../todos/ready/P6-daily-release.md))

With these three done, a merged change with a changeset reaches crates.io, npm and `docs.laminapdf.com` by the next day's release, and every docs page names the version it documents.
