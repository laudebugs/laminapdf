# Desktop update service

This Cloudflare Worker tells each installed copy of the Lamina PDF desktop app whether an update exists. The app's updater asks `https://updates.laminapdf.com/{channel}/{target}/{arch}/{current_version}`. The Worker reads that channel's manifest from R2 and answers 204 (no update) or 200 with the update's address and signature. It decides who gets a release from the manifest's staged rollout, its kill switch and its bridge releases. [ADR 0014](../../docs/decisions/0014-desktop-distribution.md) records the rules. This page covers running the service.

## Operations

The manifest tool reads and writes `channels/{channel}.json` with the package's Wrangler. It uses your `wrangler login`, or `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in the environment. The tool checks every write with the Worker's own validator and skips the write when the manifest wouldn't change. Every data centre serves a change within 60 seconds (30 s of cache plus one R2 read).

```sh
pnpm --filter @laminapdf/updates manifest show beta
pnpm --filter @laminapdf/updates manifest set-rollout stable 10          # 10 % of stable installs
pnpm --filter @laminapdf/updates manifest pause stable                   # the kill switch
pnpm --filter @laminapdf/updates manifest resume stable
pnpm --filter @laminapdf/updates manifest promote 0.2.0 --from beta --to stable --rollout 1
pnpm --filter @laminapdf/updates manifest validate my-edit.json beta
pnpm --filter @laminapdf/updates manifest put beta my-edit.json          # a hand edit, such as a bridge
```

Add `--local` to any command to work on Wrangler's local R2, which `pnpm --filter @laminapdf/updates dev` serves at `http://localhost:8787`.

**Who may change what (recommendation).** Changes to stable and beta (`promote`, `set-rollout`, `resume`, `put`) need the owner's go-ahead, normally through the desktop release workflow, which the owner approves. Anyone operating the service may `pause` a channel at once.

- **Roll out a stable release:** promote it from beta at 1 %, then move it to 10 %, 50 % and 100 %. Installs outside the rollout keep getting `previous`. As a recommendation, wait at least 24 hours at each step, and pause if any new crash signature (once crash reporting exists) or update-failure report appears.
- **Stop a bad release:** `pause` the channel. Within 60 seconds no install is offered `release` or `previous`. Bridges keep flowing.
- **Roll back:** pause the channel, then build a fixed release with a higher version. Promote that release, or, to move installs already on the bad version, add it as a bridge with `below` set above the bad version and upload it with `put`.
- **Rotate or replace an updater key:** follow [ADR 0014 §7](../../docs/decisions/0014-desktop-distribution.md#7-the-updater-keys-custody-and-rotation). The bridge is a hand-edited manifest uploaded with `put`.

To check a channel by hand:

```sh
curl -i -H "Lamina-Install-Id: $(uuidgen)" https://updates.laminapdf.com/beta/darwin/aarch64/0.1.0
```

## Updater public keys

The desktop app compiles one of these into its updater configuration, `plugins.updater.pubkey` in `tauri.conf.json` ([Tauri updater](https://v2.tauri.app/plugin/updater/)). The owner made both keys on 2026-11-12. The private halves are only in the owner's password manager, its backups, and the GitHub environments named below.

| Builds | minisign key ID | GitHub environment |
| --- | --- | --- |
| stable and beta | `AF48D217C8FBC9BB` | `desktop-release` |
| nightly | `D8B5180692ABC3EF` | `desktop-nightly` |

Stable and beta `pubkey`:

```
dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEFGNDhEMjE3QzhGQkM5QkIKUldTN3lmdklGOUpJcjVmRzd0Zjk5Ty9TTnJFMVl1NFJ1WDRlZXdKQkM3bGM2VHM5VENndUFrTHEK
```

Nightly `pubkey`:

```
dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEQ4QjUxODA2OTJBQkMzRUYKUldUdnc2dVNCaGkxMkljcG4veWhWU2x5djhqYmYyY3BSb3pMNkJYRVhsZzN5Q0pCUGk3MlFldW4K
```

A key change ships through a bridge release, since every installed copy trusts only the key it was built with.

## How it fits together

| Piece | Where |
| --- | --- |
| The Worker | `src/` (entry `src/index.ts`), `wrangler.jsonc` |
| Production | `updates.laminapdf.com`, the Worker `lamina-pdf-updates` |
| Staging | `staging-updates.laminapdf.com`, the Worker `lamina-pdf-updates-staging` (`--env staging`) |
| Stable and beta | bucket `lamina-pdf-releases`, files at `downloads.laminapdf.com` |
| Nightly | bucket `lamina-pdf-nightly`, files at `nightly-downloads.laminapdf.com` |
| Staging channel | bucket `lamina-pdf-staging`, files at `staging-downloads.laminapdf.com` |
| Channels and buckets | `src/channels.ts`, the one map; a test holds `wrangler.jsonc`'s bindings to it |
| Manifest format | `src/manifest.ts`, and `schemas/manifest.schema.json` for other tools |
| Deploys | `.github/workflows/updates-deploy.yml`, with `scripts/smoke.sh` |

The Worker only reads the buckets. A release is a set of files plus a manifest change, never a Worker deploy: the desktop release workflow uploads the bundles, and the manifest tool edits the manifest. Every push to `main` that touches the Worker deploys staging and then production, in the Worker's own `updates-production` environment, with no release tag; [ADR 0014 §5](../../docs/decisions/0014-desktop-distribution.md#5-the-r2-layout-the-manifest-and-the-endpoint) explains why that is safe.

## Owner setup

This is done once, in the Cloudflare dashboard. Until the deploy secrets exist, the deploy workflow skips with a notice.

1. **Buckets** (done 2026-11-12). In R2, create `lamina-pdf-releases`, `lamina-pdf-nightly` and `lamina-pdf-staging`.
2. **Download domains** (done 2026-11-12, minimum TLS 1.2). Under each bucket's **Settings → Custom Domains**, connect its domain, and leave the `r2.dev` address off:
   - `lamina-pdf-releases`: `downloads.laminapdf.com`
   - `lamina-pdf-nightly`: `nightly-downloads.laminapdf.com`
   - `lamina-pdf-staging`: `staging-downloads.laminapdf.com`
3. **Write tokens** (done 2026-11-12). Four R2 API tokens, each with **Bucket Item Read + Write** on one bucket only, expiring 2027-11-12:

   | Bucket | GitHub environment | Secret name |
   | --- | --- | --- |
   | `lamina-pdf-releases` | `desktop-release` | `CLOUDFLARE_API_TOKEN` |
   | `lamina-pdf-staging` | `desktop-release` | `CLOUDFLARE_STAGING_R2_TOKEN` |
   | `lamina-pdf-nightly` | `desktop-nightly` | `CLOUDFLARE_API_TOKEN` |
   | `lamina-pdf-staging` | `desktop-staging` | `CLOUDFLARE_STAGING_R2_TOKEN` |

   `CLOUDFLARE_ACCOUNT_ID` is in all three environments. The staging token in `desktop-release` serves the desktop release workflow's signed trial runs.
4. **Deploy token.** The workflow uses the repository secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`, the same ones the web reader deploys with. The token needs **Workers Scripts: Edit** on the account and **Workers Routes: Edit** on the `laminapdf.com` zone, and **no R2 access at all**: deploying a Worker with R2 bindings needs none, and only the per-bucket tokens above may write releases. The first deploy creates the `updates.` and `staging-updates.` custom domains and their certificates.
5. **Check it.** `https://updates.laminapdf.com/healthz` answers `ok`. A channel with no manifest answers 204.

The GitHub environments themselves (reviewers, branch rules, the signing secrets) are set up in the desktop release runbook, `docs/guidelines/desktop-release.md` (added by the desktop signing runbook, PR #517). The `updates-production` environment the deploy uses needs no setup: GitHub creates it on the first deploy, and it is the update Worker's own, apart from the web reader's `production`.

## Development

```sh
pnpm --filter @laminapdf/updates test        # Vitest: unit tests, the request handler in Node, the bundled Worker in workerd
pnpm --filter @laminapdf/updates typecheck   # Node and workerd types, and the generated binding types are current
pnpm --filter @laminapdf/updates run types   # regenerate src/generated/worker-configuration.d.ts after editing wrangler.jsonc
pnpm --filter @laminapdf/updates dev         # the Worker on wrangler dev with local R2
```

`cargo xtask check ts` runs the typecheck and the tests with every other package's.

Unit tests sit beside their sources (`src/*.test.ts`, `scripts/edits.test.ts`); `test/` holds the integration tests: the request handler in Node, and the bundled Worker in workerd. Together they cover every branch in ADR 0014: 204 and 200, the rollout with the install ID present, absent and malformed, `previous`, bridges, the pause, the 30-second cache bound and bad paths. A log-leak test fails if an install ID reaches any log.

The workerd tests run through Miniflare `5.20261103.0-beta`, pinned exactly: it is the Miniflare that the workspace's Wrangler 4.157.0 depends on, and Miniflare 5 has so far been published only as betas. `@cloudflare/vitest-pool-workers` would run every test in workerd, but its current release needs Vitest 4, and the workspace is on Vitest 5.

## Design notes

**Why the hosts aren't `vars`.** The Worker never builds a download URL: each manifest holds its bundles' full addresses, so moving `downloads.laminapdf.com` is a manifest change. The update endpoint's address lives in the app's updater configuration, not in the Worker, so changing it needs a bridge release whatever the Worker's settings say. The only hosts in `wrangler.jsonc` are the Worker's own routes.
