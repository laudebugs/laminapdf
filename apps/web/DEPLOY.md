# Deploying the reader

The reader runs on two Cloudflare Workers that serve `dist/` as static assets and run no code of their own, so no document can reach a server ([ADR 0011](../../docs/decisions/0011-cloudflare-topology-and-privacy.md)). `.github/workflows/web-deploy.yml` deploys them. Each reader pull request gets a preview, each merge to `main` updates staging, and only a `web-v*` tag reaches production. After every deploy, a smoke test checks the live site.

## Where each build goes

`wrangler.jsonc` describes both Workers:

- **`lamina-pdf-web`**, at **https://app.laminapdf.com** (a Workers custom domain): production. It has no workers.dev address. Its preview URLs, on workers.dev, hold the per-PR previews.
- **`lamina-pdf-web-staging`** (`env.staging`), at **https://staging-app.laminapdf.com**: `main`'s reader.

| Event | Jobs | Deploys to |
| --- | --- | --- |
| A pull request from this repository | `preview-build`, `preview-deploy`, `preview-smoke` | A preview version of `lamina-pdf-web` under the alias `pr-<number>`, at the URL commented on the PR |
| A push to `main`, or a manual run on `main` | `staging` | `staging-app.laminapdf.com` |
| A tag `web-v<version>` | `production-checks`, `production` | `app.laminapdf.com` |

A pull request or a push deploys only when it changes something the reader is built from: the app, `packages/`, `crates/`, `fixtures/`, `tools/xtask/`, the lockfiles, the root manifests, the toolchain pins or the workflow itself. `cargo xtask changes` decides, from one list (`WEB_INPUTS` in `tools/xtask/src/changes.rs`), and prints `web=true` or `web=false`. Otherwise the job reports success and its summary says why nothing deployed. A manual run on `main` always deploys staging, and a tag always deploys production.

## How a preview is built and deployed

The preview jobs keep the PR's code away from the Cloudflare secrets:

1. **`preview-build`** has no secrets. It builds the PR with `cargo xtask web build` and uploads `apps/web/dist/` as an artifact.
2. **`preview-deploy`** has the secrets and never checks out or runs the PR's code. It takes `wrangler.jsonc` and the Wrangler version from the default branch, and installs that exact Wrangler with `npx` into a fresh npm cache. Then it uploads the artifact with `wrangler versions upload --preview-alias pr-<number>`, which leaves production as it is. It measures the build's JavaScript and WebAssembly (gzipped, file by file) and posts one comment on the PR with the preview URL, the commit and those sizes, updated on every push.
3. **`preview-smoke`** runs the PR's smoke test against the preview URL, with no secrets.

`versions upload` refuses a Worker that doesn't exist yet, and production's first deploy waits for the first `web-v*` tag. So the first preview creates `lamina-pdf-web` itself. It deploys a placeholder page with no route and no workers.dev address, and turns preview URLs on. `app.laminapdf.com` stays unattached until that first tag.

Pull requests from forks are skipped, as in the docs and website workflows, because they must never run on the self-hosted runners or see the secrets.

## Production and rollbacks

A `web-v<version>` tag runs two jobs, one after the other:

1. **`production-checks`** builds the tag, typechecks the reader and runs its e2e suite in Chromium against `wrangler dev`. Todo C7c adds the release gates to this job: the runtimes, Lighthouse and installability.
2. **`production`** needs those checks. It deploys the build they passed, in the `production` GitHub environment, then runs the smoke test against `app.laminapdf.com`. If the smoke test fails, the job runs `wrangler rollback`, which restores the previous deployment, and fails.

Nothing else deploys production: a push to a branch never does. Production deploys run one at a time, and so do staging deploys; a newer push to a pull request cancels its older preview run.

## What the owner needs to do

1. **The secrets.** The workflow uses the repository secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`, which the docs site and the website already use. The token needs:
   - **Account → Workers Scripts → Edit**: creates the Workers, uploads versions, deploys and rolls back.
   - **Account → Account Settings → Read**: lets Wrangler read the account it deploys to.
   - **Zone → Workers Routes → Edit** on the `laminapdf.com` zone: attaches `app.laminapdf.com` and `staging-app.laminapdf.com`. If a deploy still fails with a permission error on a domain, add **Zone → DNS → Edit** on the same zone.

   Without the secrets, every deploy step skips with a notice and the jobs pass.
2. **The workflow file.** The agents' GitHub access can't push workflow files, so the owner commits `.github/workflows/web-deploy.yml`.
3. **DNS.** The first staging deploy creates the Worker `lamina-pdf-web-staging`, the `staging-app` DNS record and its certificate; the first `web-v*` tag does the same for `app`. Each fails if a DNS record with that name already exists, so delete any first.
4. **Zone settings.** A zone setting that injects a script into pages, such as Rocket Loader or Email Address Obfuscation, breaks the reader: the CSP blocks the script, and the smoke test fails on the violation. Keep such settings off for these hosts.
5. **Optional: production reviewers.** The `production` job runs in the GitHub environment `production`, which GitHub creates on first use. To approve each production deploy by hand, add required reviewers to it (Settings → Environments → production).

Nothing needs creating in the Cloudflare dashboard.

## Previews after a pull request closes

Wrangler has no command to delete a preview alias, so a closed pull request's preview stays reachable until Cloudflare drops its version from the Worker's history. A preview holds only the reader's public files, so a stale one exposes nothing.

## Deploying and rolling back by hand

For an emergency, deploy from a checkout of the tag you want live. Wrangler needs the same token and account, as `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in the environment, or a `wrangler login`. First build the reader, which needs the WebAssembly tools from [the README](README.md#run-it):

```sh
cargo xtask web build
```

Then deploy production, or staging with `--env staging`:

```sh
pnpm --filter @laminapdf/web exec wrangler deploy --env="" --message "web-v0.1.1 by hand"
```

To put the previous deployment back, roll back:

```sh
pnpm --filter @laminapdf/web exec wrangler rollback --env="" --message "why"
```

`wrangler deployments list --env=""` lists the recent deployments, and `wrangler rollback <version-id>` restores an older one. Rerunning the workflow's `staging` job (Actions → Web reader → Run workflow, on `main`) redeploys staging from `main`.

## Checking a deployed reader

The smoke test runs against any deployed reader, in Chromium:

```sh
pnpm --filter @laminapdf/web exec playwright install chromium
LAMINA_PDF_SMOKE_URL=https://staging-app.laminapdf.com pnpm --filter @laminapdf/web run smoke
```

It checks that `/` and an unknown path return the app with ADR 0011's headers exactly, that the page's hashed script is cached for good, and that `hello.pdf`, opened with **Choose a file**, renders page 1. It fails on any CSP violation and on any request to another origin.
