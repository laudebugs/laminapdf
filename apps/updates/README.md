# Desktop update service

This Cloudflare Worker tells each installed copy of the Lamina PDF desktop app whether an update exists. The app's updater asks `https://updates.laminapdf.com/{channel}/{target}/{arch}/{current_version}`. The Worker reads that channel's manifest from R2 and answers 204 (no update) or 200 with the update's address and signature. It decides who gets a release from the manifest's staged rollout, its kill switch and its bridge releases. [ADR 0014](../../docs/decisions/0014-desktop-distribution.md) records the rules. This page covers running the service.

## How it fits together

| Piece | Where |
| --- | --- |
| The Worker | `src/` (entry `src/index.ts`), `wrangler.jsonc` |
| Production | `updates.laminapdf.com`, the Worker `lamina-pdf-updates` |
| Staging | `staging-updates.laminapdf.com`, the Worker `lamina-pdf-updates-staging` (`--env staging`) |
| Stable and beta | bucket `lamina-pdf-releases`, files at `downloads.laminapdf.com` |
| Nightly | bucket `lamina-pdf-nightly`, files at `nightly-downloads.laminapdf.com` |
| Staging channel | bucket `lamina-pdf-staging`, files at `staging-downloads.laminapdf.com` |
| Manifest format | `src/manifest.ts`, and `schemas/manifest.schema.json` for other tools |
| Deploys | `.github/workflows/updates-deploy.yml`: every push to `main` that touches the Worker deploys staging, smoke-tests it (`scripts/smoke.sh`), then deploys production |

The Worker only reads the buckets. A release is a set of files plus a manifest change, never a Worker deploy: D6b's release workflow uploads the bundles, and the manifest tool below edits the manifest.

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

A key change ships through a bridge release ([ADR 0014 §7](../../docs/decisions/0014-desktop-distribution.md#7-the-updater-keys-custody-and-rotation)), since every installed copy trusts only the key it was built with.

## Operations

The manifest tool reads and writes `channels/{channel}.json` with the package's Wrangler. It uses your `wrangler login`, or `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in the environment. Every write is checked with the Worker's own validator first, and nothing is written when the manifest wouldn't change. The Worker serves a change within 30 seconds.

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

- **Roll out a stable release:** promote it from beta at 1 %, then move it to 10 %, 50 % and 100 % while crash reports and feedback stay quiet. Installs outside the rollout keep getting `previous`.
- **Stop a bad release:** `pause` the channel. Within a minute no install is offered `release` or `previous`. Bridges keep flowing.
- **Roll back:** build a fixed release with a higher version, then promote it, or add it as a bridge for every version that needs it with `put`.
- **Rotate or replace an updater key:** follow ADR 0014 §7. The bridge is a hand-edited manifest uploaded with `put`.

To check a channel by hand:

```sh
curl -i -H "Lamina-Install-Id: $(uuidgen)" https://updates.laminapdf.com/beta/darwin/aarch64/0.1.0
```

## Development

```sh
pnpm --filter @laminapdf/updates test        # Vitest: the handler in Node, the bundled Worker in workerd
pnpm --filter @laminapdf/updates typecheck   # Node and workerd types, and the generated binding types are current
pnpm --filter @laminapdf/updates run types   # regenerate src/generated/worker-configuration.d.ts after editing wrangler.jsonc
pnpm --filter @laminapdf/updates dev         # the Worker on wrangler dev with local R2
```

`cargo xtask check ts` runs the typecheck and the tests with every other package's. The tests cover every branch in ADR 0014: 204 and 200, the rollout with the install ID present, absent and malformed, `previous`, bridges, the pause, the 30-second cache bound, bad paths, and a log-leak test that fails if an install ID reaches any log.

## Owner setup

This is done once, in the Cloudflare dashboard. Until the deploy secrets exist, the deploy workflow skips with a notice.

1. **Buckets.** In R2, create `lamina-pdf-releases`, `lamina-pdf-nightly` and `lamina-pdf-staging`.
2. **Download domains.** Under each bucket's **Settings → Custom Domains**, connect `downloads.laminapdf.com`, `nightly-downloads.laminapdf.com` and `staging-downloads.laminapdf.com` respectively. Leave the `r2.dev` address off.
3. **Write tokens, one per bucket.** Create an R2 API token with **Object Read & Write** on that bucket only, and store it in a GitHub environment as `CLOUDFLARE_API_TOKEN`, with `CLOUDFLARE_ACCOUNT_ID` beside it:
   - `lamina-pdf-releases` in `desktop-release`
   - `lamina-pdf-nightly` in `desktop-nightly`
   - `lamina-pdf-staging` in `desktop-staging` as `CLOUDFLARE_STAGING_R2_TOKEN`, and again in `desktop-release` for D6b–D6d's signed trial runs
4. **Deploy token.** The workflow uses the repository secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`, the same ones the web reader deploys with. The token needs **Workers Scripts: Edit** on the account and **Workers Routes: Edit** on the `laminapdf.com` zone. The first deploy creates the `updates.` and `staging-updates.` custom domains and their certificates.
5. **Check it.** `https://updates.laminapdf.com/healthz` answers `ok`. A channel with no manifest answers 204.

The GitHub environments themselves (reviewers, branch rules, the signing secrets) are set up in D6a's runbook.
