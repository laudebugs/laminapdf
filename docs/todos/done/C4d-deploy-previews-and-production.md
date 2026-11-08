# C4d — Deploy pipeline: preview URL per PR, staging from `main`, production from release tags

| Field | Value |
| --- | --- |
| ID | `C4d` |
| Status | `done` |
| Plan | [web reader §6 C4. App shell on Cloudflare](../../plans/web-app.md#c4-app-shell-on-cloudflare) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C4b](./C4b-web-app-shell.md), [W1a](./W1a-domain-map.md) |
| ADRs | Implements 0011 (Wrangler pinned in CI, previews per PR, staging on `main`, production from `web-v*` tags); follows [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (pinned actions, least-privilege tokens) |
| Branch | `ci/c4d-web-deploys` |
| PR title | `ci(web): deploy a preview per pull request, staging from main and production from release tags` |
| Size | M (200–600 lines of workflow, scripts and docs) |
| PR | [#303](https://github.com/laudebugs/laminapdf/pull/303) |

## Goal

Every PR that touches the reader gets a working preview URL commented on the PR, every merge to `main` deploys **staging**, and **production deploys only from a `web-v*` release tag after the release lane passes** (so C7c's release gates can actually block production), all through a pinned Wrangler in GitHub Actions. After deploy, a smoke test checks the live headers and that a fixture renders. Reviewers can then try every reader PR from C5 onward in a real browser on real Cloudflare.

## Context

- ADR 0011 (from [C2a](../done/C2a-render-adrs.md)) — production at `app.laminapdf.com`, staging at `staging-app.laminapdf.com`, previews per PR via Workers preview URLs, Wrangler pinned.
- [cloudflare notes §1](../../research/architecture/notes/cloudflare.md) — Workers needs `preview_urls: true` and a `previews` block; previews at `<preview-name>.<your-domain>`; custom domain must be a Cloudflare zone; unverified whether Workers Builds creates PR previews automatically; `cf` CLI is beta (try later).
- [Git workflow §5–7](../../guidelines/git-workflow.md#5-pull-requests) — PR process; CI must be green.
- [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) — path-filtering inside jobs, never workflow-level `paths` on required checks.

## Scope

**In scope**

- `.github/workflows/web-deploy.yml`: on `pull_request` (same-repo branches only; forks get no secrets → skip with a note) build `apps/web` and `wrangler versions upload` with a preview alias derived from the PR number (e.g. `pr-123`), then comment (create or update a single sticky comment) with the preview URL; on `push` to `main` build and deploy **staging** (a stable `staging` preview alias or a separate `lamina-pdf-web-staging` Worker — decide by what the pinned Wrangler supports); on `push` of a tag `web-v*` deploy **production**, with the production job `needs:` the release-lane gate jobs (C7c adds the runtime/Lighthouse/installability gates to that lane; until then it needs the build + e2e jobs) so a failing gate means no production deploy; concurrency groups per PR, for staging and for production. Nothing deploys production from a branch push.
- `wrangler.jsonc`: `preview_urls: true`, environment/route config for the custom domains `app.laminapdf.com` and `staging-app.laminapdf.com` ([web-app §8 Q1](../../plans/web-app.md#8-open-questions); the docs site's `apps/docs/wrangler.toml` shows the `custom_domain` route), `workers_dev` policy.
- Secrets: `CLOUDFLARE_API_TOKEN` (scoped to Workers Scripts edit on the one account) and `CLOUDFLARE_ACCOUNT_ID`, documented; GitHub environment `production` with required reviewers optional.
- Post-deploy smoke (`apps/web/e2e/smoke.spec.ts` with `BASE_URL`): headers match ADR 0011, SPA fallback works, `hello.pdf` (served from the test, opened via file input) renders page 1.
- Preview cleanup policy (previews expire or are deleted when the PR closes, if the API allows; otherwise document retention).

**Out of scope** (and which todo covers it)

- The `web v0.1` tag and production launch checklist → [C7c](../ready/C7c-performance-gates-and-launch.md).
- Docs-site deploys (library B7).
- Migration to the `cf` CLI (later, after GA).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`. Claimed by opening the work PR, as git workflow §2 now says.
- [x] Create the Cloudflare Worker `lamina-pdf-web` (owner action: account, API token with minimal scope; record the steps in `apps/web/DEPLOY.md`, no secrets in the repo). The workflow creates all three Workers itself: the previews' Worker `lamina-pdf-web-preview` on the first preview, staging's and production's on their first deploys. `apps/web/DEPLOY.md` records the token's permissions.
- [x] Add `preview_urls`/`previews` config to `wrangler.jsonc`; verify `wrangler versions upload --preview-alias pr-<n>` (or the current equivalent for the pinned Wrangler version) yields a stable URL per PR. Previews live on their own Worker, `lamina-pdf-web-preview`, whose config the workflow writes from `wrangler.jsonc`'s top-level settings, so production has no preview URLs. `wrangler versions upload --preview-alias pr-<n>` gives one URL per PR.
- [x] Write `web-deploy.yml`: checkout, set up pnpm/Node (versions from `.nvmrc`/`package.json`), Rust toolchain + `cargo xtask wasm` (the app needs the WASM builds), `pnpm --filter @laminapdf/web build` (package name from C4b; the step fails if `apps/web/dist/index.html` is missing afterwards), upload/deploy with the pinned Wrangler from the lockfile (`pnpm exec wrangler`), actions pinned by SHA. The build is `cargo xtask web build` (the WebAssembly modules without their tests, then the packages and the app; it fails without `dist/index.html`). The preview upload runs the default branch's pinned Wrangler with `npx`, as `docs.yml` does; staging and production use `pnpm exec wrangler`.
- [x] Sticky PR comment via `gh`/`actions/github-script`: preview URL, commit SHA, build size summary (JS min+gz, WASM gz — used later by C7c). Through `gh api`: the URL, the commit, and the JavaScript and WebAssembly each gzipped file by file (Vite's JavaScript is already minified).
- [x] Path filtering inside the **preview and staging** jobs only (deploy when any of these changed: `apps/web/`, `packages/`, `crates/`, `fixtures/`, `tools/xtask/`, the lockfiles `pnpm-lock.yaml` and `Cargo.lock`, the root manifests `Cargo.toml`, `package.json`, `pnpm-workspace.yaml`, `tsconfig.base.json`, `.nvmrc`, `rust-toolchain.toml`, and `.github/workflows/web-deploy.yml` itself); the job reports success otherwise. The **production job is never path-filtered** (it runs for every `web-v*` tag). The filter list lives in one place (a shared paths file or a reusable step) that C7c's size gate reuses. Inverted: `cargo xtask changes` prints `web=true` for every path but the docs and `OUTSIDE_WEB` in `tools/xtask/src/changes.rs` (the other apps and workflows, the tools other than xtask, the spikes, `fuzz/`), so `.npmrc`, `.cargo/config.toml` and the shared actions count too. Staging compares a push with the workflow's last successful run on `main`.
- [x] Staging job on `main` with the smoke test against the staging URL. Production job on `web-v*` tags only, `environment: production`, `needs:` the release-lane gates, smoke test against the production URL; on smoke failure, roll back with `wrangler rollback` and fail the job. Until C7c, `production` needs `production-checks` (the build, the typecheck and the e2e suite in Chromium, on a tag that is a commit of `main`); the rollback runs whenever the smoke test doesn't pass.
- [x] Smoke spec parameterised by `BASE_URL`; run against the preview in PR runs. `apps/web/e2e/smoke.test.ts` with `LAMINA_PDF_SMOKE_URL`; `e2e/live.ts` first waits up to 10 minutes for the host.
- [x] `apps/web/DEPLOY.md`: setup, secrets, how previews and rollbacks work, how to deploy by hand in an emergency.

## Acceptance criteria

- [x] This PR itself receives a preview URL comment, and the smoke test passes against it. The first preview ran on Cloudflare and its smoke test passed.
- [x] Pushing a second commit updates the same comment (not a new one). The one comment was updated by each later run.
- [x] A PR that changes only `pnpm-lock.yaml` (or only `Cargo.lock`, or only `web-deploy.yml`) gets a preview (path filter covers lockfiles, root manifests and the workflow file). `touches_web`'s tests cover the lockfiles, the root config and the workflow file.
- [ ] The production job is exercised before merge in dry-run form: on a test branch with the job's tag-only condition temporarily widened and the deploy target switched to a throwaway Worker name, a forced failing smoke triggers `wrangler rollback` and fails the job (documented in the PR, then reverted). Not done: the rehearsal needs a temporary edit to the workflow, which only the owner can push. A `workflows.rs` test pins the rollback condition instead.
- [x] Fork PRs don't fail CI for lack of secrets (job skipped with an explanatory summary). Fork PRs are skipped, as in `docs.yml` and `site.yml`, without a summary: a summary job for forks would need a GitHub-hosted runner.
- [x] Wrangler version is pinned (lockfile) and actions are pinned by SHA.
- [x] Live headers on the preview match ADR 0011 (smoke asserts them).

## Post-merge checks

- [ ] Merging deploys **staging** from `main` (not production); the smoke test runs against the staging URL and passes, and live headers there match ADR 0011.
- [ ] Production is first deployed by C7c's `web-v0.1.0` tag (checked there); no production deploy happens from this merge.

## Tests

`apps/web/e2e/smoke.spec.ts` (Chromium only for speed in deploy jobs; full browser matrix stays in the e2e lane). Workflow linting with `actionlint` if available in CI.

## Docs to update

`apps/web/DEPLOY.md`, `apps/web/README.md`, `CLAUDE.md` (deploy commands), [git workflow](../../guidelines/git-workflow.md) only if the review process mentions previews (add one line: "reader PRs: check the preview URL"); tick "Add CI deploy: preview URL per PR (comment on the PR), production on `main`" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Domain** (`laminapdf.com`, [§8 Q1](../../plans/web-app.md#8-open-questions); the reader's hosts are `app.` and `staging-app.`, the apex is the website's, [ADR 0011 §1](../../decisions/0011-cloudflare-topology-and-privacy.md#1-topology)): the zone is on Cloudflare; the deploy token needs Zone → Workers Routes → Edit on it, as the docs site's does.
- **Plan wording**: [web-app.md](../../plans/web-app.md) still says "production on `main`"; this todo supersedes that with staging on `main` + production from tags; tick the §9 item with a note.
- **Preview-URL mechanics** for Workers changed recently and are partly unverified in the notes; follow current Wrangler docs for the pinned version and record exact commands.
- **Secrets on PRs**: only same-repo branches deploy; never use `pull_request_target` with checkout of PR code.
- **Workers Logs** must not log full request URLs with sensitive data; v0.1 has no Worker code and fragments never reach the server, but confirm observability settings per ADR 0011.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started (the open work PR is the claim, git workflow §2)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings) 10 findings, then 3 on the second review, all fixed; `/writing-review` 13 + 1.
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
