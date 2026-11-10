# Deploying the reader

The reader runs on Cloudflare Workers that serve `dist/` as static assets and run no code of their own, so no document can reach a server ([ADR 0011](../../docs/decisions/0011-cloudflare-topology-and-privacy.md)). `.github/workflows/web-deploy.yml` deploys them. Each reader pull request gets a preview, each merge to `main` updates staging, and only a `web-v*` tag reaches production. After every deploy, a smoke test checks the live site.

## Where each build goes

Three Workers hold the reader, each at one kind of address:

- **`lamina-pdf-web`**, at **https://app.laminapdf.com** (a Workers custom domain): production, from `wrangler.jsonc`'s top-level settings. It has no workers.dev address and no preview URLs, so production has one address.
- **`lamina-pdf-web-staging`** (`env.staging`), at **https://staging-app.laminapdf.com**: `main`'s reader.
- **`lamina-pdf-web-preview`**: the per-PR previews, each at its own preview URL on workers.dev. The workflow writes its config from `wrangler.jsonc`'s top-level settings, with that name, no route, no workers.dev address and preview URLs on. The one page it deploys, a placeholder, has no address.

| Event | Jobs | Deploys to |
| --- | --- | --- |
| A pull request from this repository | `preview-build`, `preview-deploy`, `preview-smoke` | A preview version of `lamina-pdf-web-preview` under the alias `pr-<number>`, at the URL commented on the PR |
| A push to `main`, or a manual run on `main` | `staging` | `staging-app.laminapdf.com` |
| A tag `web-v<version>` on a commit of `main`: the web release train's, once a day when the reader changed, or one pushed by hand | `production-checks`, `production` | `app.laminapdf.com` |

A pull request or a push deploys only when it changes something the reader may be built from. `cargo xtask changes` decides and prints `web=true` or `web=false`. It counts every path except the docs and `OUTSIDE_WEB` in `tools/xtask/src/changes.rs`, the parts of the repository the reader can't depend on. Those are the other apps and workflows, the tools other than xtask, the spikes and the fuzz workspace. A root config file or a shared action nobody thought to list still counts. When nothing it counts changed, the job reports success and its summary says why nothing deployed.

A pull request compares itself with its base branch. A push to `main` compares itself with the commit of the workflow's last successful run on `main`, which deployed staging or found nothing to deploy. So the next push makes up for a cancelled or failed staging deploy, whatever that push changes. A manual run on `main` always deploys staging, and a tag always deploys production.

## How a preview is built and deployed

The preview jobs keep the PR's code away from the Cloudflare secrets:

1. **`preview-build`** has no secrets. It builds the PR with `cargo xtask web build` and uploads `apps/web/dist/` as an artifact.
2. **`preview-deploy`** has the secrets and never checks out or runs the PR's code. It reads `wrangler.jsonc` and the Wrangler version from the default branch. Everything else happens in the runner's temp folder, away from the workspace where a PR's job left its checkout. There, `npx` runs that exact Wrangler with a fresh npm cache and an empty npm config. The job writes the preview Worker's config, then uploads the artifact with `wrangler versions upload --preview-alias pr-<number>`, which deploys nothing. Last, it measures the build's JavaScript and WebAssembly (gzipped, file by file) and posts one comment on the PR with the preview URL, the commit and those sizes, updated on every push.
3. **`preview-smoke`** runs the PR's smoke test against the preview URL, with no secrets.

`versions upload` needs an existing Worker, so the first preview creates `lamina-pdf-web-preview`: it deploys a placeholder page with no route and no workers.dev address, and turns preview URLs on. Production is untouched until the first `web-v*` tag.

Pull requests from forks are skipped, as in the docs and website workflows, because they must never run on the self-hosted runners or see the secrets.

## Production and rollbacks

A `web-v<version>` tag runs two jobs, one after the other:

1. **`production-checks`** fails unless the tag is on a commit of `main`, which takes changes only through pull requests that passed CI, and unless its version suits `package.json`'s (`cargo xtask web check-tag`). It builds the reader with the tag's version, which Settings › About shows. It builds the tag, typechecks the reader and runs its e2e suite in Chromium against `wrangler dev`. Todo C7c adds the release gates to this job: the runtimes, Lighthouse and installability.
2. **`production`** needs those checks. It deploys the build they passed, in the `production` GitHub environment, then runs the smoke test against `app.laminapdf.com`. Once the smoke test passes, it publishes the tag's GitHub release with the tag's notes. Whenever the smoke test doesn't pass, the job runs `wrangler rollback`, which restores the previous deployment, and fails; a smoke test that times out or is cancelled counts too. The first release has no deployment to go back to, so its rollback fails too, and the release stays up until a fixed tag replaces it.

`production-checks` takes longer than a preview build. A tag's run builds everything from scratch, because `isolate-rust-home` and `node-pnpm` empty its build directory and package store before every use.

Only a `web-v*` tag deploys production. Deploys to one Worker run one at a time, and a newer push to a pull request cancels its older preview run.

## What the owner needs to do

1. **The secrets.** The workflow uses the repository secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`, which the docs site and the website already use. The token needs:
   - **Account → Workers Scripts → Edit**: creates the three Workers, uploads versions, deploys and rolls back.
   - **Account → Account Settings → Read**: lets Wrangler read the account it deploys to.
   - **Zone → Workers Routes → Edit** on the `laminapdf.com` zone: attaches `app.laminapdf.com` and `staging-app.laminapdf.com`. If a deploy still fails with a permission error on a domain, add **Zone → DNS → Edit** on the same zone.

   Without the secrets, every deploy step skips with a notice and the jobs pass.
2. **The workflow file.** The agents' GitHub access can't push workflow files, so the owner commits `.github/workflows/web-deploy.yml`.
3. **DNS.** The first staging deploy creates the Worker `lamina-pdf-web-staging`, the `staging-app` DNS record and its certificate; the first `web-v*` tag does the same for `lamina-pdf-web` and `app`. Either deploy fails if a DNS record with its name already exists, so delete any such record first.
4. **Zone settings.** A zone setting that injects a script into pages, such as Rocket Loader or Email Address Obfuscation, breaks the reader: the CSP blocks the script, and the smoke test fails on the violation. Keep such settings off for these hosts.
5. **Optional: production reviewers.** The `production` job runs in the GitHub environment `production`, which GitHub creates on first use. To approve each production deploy by hand, add required reviewers to it (Settings → Environments → production).

The workflow creates all three Workers itself, so the Cloudflare dashboard needs no setup.

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

`wrangler deployments list --env=""` lists the recent deployments, and `wrangler rollback <version-id>` restores an older one. To redeploy staging from `main`, run the workflow by hand on `main` (Actions → Web reader → Run workflow).

## Checking a deployed reader

The smoke test runs against any deployed reader, in Chromium:

```sh
pnpm --filter @laminapdf/web exec playwright install chromium
LAMINA_PDF_SMOKE_URL=https://staging-app.laminapdf.com pnpm --filter @laminapdf/web run smoke
```

First `e2e/live.ts` waits, for up to 10 minutes, until the address serves the app: a custom domain's first deploy can take minutes to get its DNS record and certificate. Then it checks that `/` and an unknown path return the app with ADR 0011's headers exactly, that the page's hashed script is cached for good, and that `hello.pdf`, opened with **Choose a file**, renders page 1. It fails on any CSP violation and on any request to another origin.

## Troubleshooting

### The smoke test is refused (HTTP 401, 403 or 407)

When something other than Cloudflare refuses the smoke test three times in a row, `e2e/live.ts` stops waiting, because waiting won't change that refusal. Cloudflare's own refusals are waited out for the full 10 minutes, since a new custom domain can answer 403 while it is set up. The script's error names the runner and lists the answer's `server`, `cf-ray`, `cf-mitigated` and `via` headers, so the error tells you who refused:

- **No `cf-ray` header.** Cloudflare stamps every answer it gives with `cf-ray`, so a proxy or security gateway on the runner's machine sent the refusal. The error also lists any proxy settings the machine sets, such as `HTTPS_PROXY` or `NODE_TLS_REJECT_UNAUTHORIZED`. The previews live on new `pr-<number>-lamina-pdf-web-preview.<account>.workers.dev` hosts, which gateways often block as newly seen domains. If the smoke test ever runs on a machine behind such a gateway, allow `*.workers.dev` and `*.laminapdf.com` in the gateway, or run the smoke-test jobs elsewhere.
- **A `cf-ray` header.** Cloudflare refused. Look for Cloudflare Access on the Worker's workers.dev and preview URLs (the Worker's **Settings → Domains & Routes**), or a WAF or bot rule on the custom domain. A `cf-mitigated: challenge` header means a bot challenge.

For example, in November 2026, when the workflow ran on self-hosted Macs, the preview smoke test was refused only on one Mac's runners (`m16r*`), whose environment sets `NODE_TLS_REJECT_UNAUTHORIZED=0`, while the same preview served the reader everywhere else. The workflow now runs on GitHub-hosted runners.
