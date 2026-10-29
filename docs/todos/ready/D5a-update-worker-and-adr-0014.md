# D5a — ADR 0014 and the `apps/updates` Worker: channels, staged rollout, kill switch

| Field | Value |
| --- | --- |
| ID | `D5a` |
| Status | `ready` |
| Plan | [Desktop app §6 D5](../../plans/desktop-app.md#d5-update-service) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C2a](./C2a-render-adrs.md), [C4d](./C4d-deploy-previews-and-production.md), [D2a](./D2a-adr-0008-and-desktop-scaffold.md), [D3b](./D3b-recent-files-and-file-watching.md) |
| Blocks | [D5b](./D5b-updater-in-app.md), [D6a](./D6a-signing-runbook-and-owner-setup.md), [D7a](./D7a-crash-reporting.md) |
| Dependency notes | [D2a](D2a-adr-0008-and-desktop-scaffold.md) (bundle identifier, product name, target/arch matrix); web plan **C4** (`apps/web` Worker conventions, Wrangler pin, deploy workflow, [web-app §C4](../../plans/web-app.md#c4-app-shell-on-cloudflare)) and ADR 0011 from web **C2** (Cloudflare topology, privacy and logging rules). Ordered after [D3b](D3b-recent-files-and-file-watching.md) on the board (see Depends on), though it uses no D3 code; it does not need D4 and may run in parallel with D4a–D4c. |
| ADRs | **Writes ADR 0014** (desktop distribution). Follows ADR 0011, [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md), ADR 0017 (attestations, library plan) |
| Branch | `feat/d5a-update-worker` |
| PR title | `feat(updates): add ADR 0014 and the update Worker with channels and staged rollout` |
| Size | L (700–1,200 lines incl. ADR and tests) |
| PR | — |

## Goal

Decide and build the server side of desktop updates. ADR 0014 records channels, staged rollout, the kill switch, the anonymous install-ID header, the R2 layout and manifest schema, signing identities (the owner personally as publisher on all three OSes) and — most critically — custody and rotation of the Tauri updater signing key, whose loss strands every installed copy. `apps/updates` is a Cloudflare Worker answering the Tauri updater's dynamic endpoint `GET /{channel}/{target}/{arch}/{current_version}` with **204 No Content** or **200** JSON, reading channel manifests from R2, bucketing installs by a hash of the `Lamina-Install-Id` header so a release can go 1 % → 10 % → 50 % → 100 %, and honouring a kill switch within 60 s.

## Context

- [Desktop plan §5 distribution](../../plans/desktop-app.md#5-design-summary): the full design (R2 `releases/{version}/{target}-{arch}/` with `.sig`; channel manifests; `updates.<domain>/{channel}/{target}/{arch}/{current_version}`; downloads from `downloads.<domain>`; install-ID rules: random UUID on first run, stored only in app config, resettable, never linked to crash reports, hashed with the release version, never logged; requests without the header get only 100 % releases).
- [Desktop plan §4 non-functional](../../plans/desktop-app.md#non-functional-targets-and-release-gates): rollout 1/10/50/100 %, kill switch within 60 s. [§4 req. 8](../../plans/desktop-app.md#functional): managed installs can pin a channel.
- [Architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs) (ADR 0014 scope), [§6 risks](../../plans/architecture.md#6-risks) (signing identities and keys), [§8 Q5](../../plans/architecture.md#8-open-questions-for-the-team) (owner signs personally).
- [Cloudflare notes](../../research/architecture/notes/cloudflare.md) (R2 pricing/limits, custom domains, 1 write/s per key; KV/R2 consistency; Workers static/route config) and [desktop notes Q5](../../research/architecture/notes/desktop-shell.md#q5-auto-update-and-hosting-on-cloudflare-r2--workers-delta-updates) (updater formats: mandatory signatures; dynamic endpoint 200/204; `{{target}}`, `{{arch}}`, `{{current_version}}`; full bundles only; R2 must be fetched through a public custom domain).
- [Report: Cloudflare desktop distribution](../../research/architecture/app-and-package-architecture.md#cloudflare-hosts-everything-for-about-5-a-month-with-heavy-jobs-in-containers): key custody belongs in an ADR.
- Tauri: [updater plugin](https://v2.tauri.app/plugin/updater/) — dynamic response `{ "version", "pub_date", "url", "signature", "notes" }`; `{{target}}` ∈ `linux|windows|darwin`, `{{arch}}` ∈ `x86_64|aarch64|i686|armv7`; `UpdaterBuilder::header` / JS `check({ headers })`; `version_comparator` for rollbacks.
- `workers-best-practices` and `wrangler` skills (load them for the Worker code).

## Scope

**In scope**

- `docs/decisions/0014-desktop-distribution.md` (**Accepted** on merge):
  1. Channels `stable`, `beta`, `nightly` (and `staging` for tests, not user-selectable); default channel for new installs (plan open question 4 — recommend `stable`, with beta opt-in; owner decides); managed pinning (macOS configuration profile key, Windows registry `HKLM\Software\Policies\Lamina\UpdateChannel` / `DisableUpdates`, Linux `/etc/lamina/policy.json`) — implemented in D5b.
  2. Rollout algorithm: `bucket = u32_be(SHA-256(install_id_lowercase ‖ ":" ‖ release_version)[0..4]) mod 10_000`; eligible if `bucket < rollout_percent × 100`. Hashing with the version reshuffles cohorts per release. No header or malformed header → only releases at 100 %.
  3. Install ID: UUID v4, created on first run, stored only in the app's config, resettable, sent only as `Lamina-Install-Id` to the update endpoint, never to the crash endpoint, never logged or stored by the Worker (and how that's verified).
  4. Kill switch semantics (effective within 60 s worst case: one cache layer with a 30 s TTL, see below): per-channel `paused: true` → 204 for every client that no bridge covers: **bridges bypass `paused`**, so a channel can be frozen for normal releases while a bridge is being served (the key-compromise and bad-release procedures below need exactly that); per-release `rollout_percent: 0`; emergency rollback via a "bridge" release with a higher version (preferred) or Tauri's `version_comparator` (requires app support; documented). Stopping a bad bridge means removing it from `bridges` (same 60 s bound).
  5. R2 layout (below), manifest schema (below), cache rules (a single Cache API layer, TTL 30 s, no isolate memo), downloads domain, CORS not needed (the updater is native HTTP).
  6. Signing identities: the owner as publisher — Apple Developer ID (individual), Windows Artifact Signing if eligible (US/Canada individuals) else OV certificate in a cloud HSM, Linux GPG key for AppImage/deb (/rpm) — details and setup in D6a's runbook.
  7. **Key custody and rotation**: Tauri updater key (minisign/ed25519) generated offline with `pnpm tauri signer generate -w <path>` with a strong password; private key in a password manager/secret manager **plus** an offline encrypted backup (two physical locations); CI receives it only as a GitHub **environment** secret (`release` environment with required reviewer = owner) as `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. **Nightly builds use a separate nightly-only updater key** (its own keypair, stored only in the `nightly` environment, which has no required reviewer so scheduled builds aren't blocked — D6a/D6b): nightly builds embed the nightly public key and only ever install nightly-signed bundles, so a leaked nightly key can't push an update to beta/stable installs; nightly is therefore a separate install line (switching between nightly and beta/stable means reinstalling — D5b). Rotation of the nightly key follows the same runbook; rotation runbook: ship release N (signed with the old key) whose config embeds the new public key; all later releases signed with the new key; the Worker serves N as a bridge to any `current_version < N` indefinitely; compromise procedure: pause the affected channels (stops normal releases signed with the compromised key; bridges still flow), publish a bridge covering every `current_version` below it (installed copies trust only the old key, so the bridge is signed with the old key and embeds the new public key; later releases use the new key). Risk the owner accepts in ADR 0014: until every install has taken the bridge, whoever holds the leaked key can also sign updates those installs accept. Then revoke the leaked key and notify users.
  8. Attestations (GitHub artifact attestations per ADR 0017) and SBOMs per release (ADR 0019).
  9. Privacy: what the update endpoint sees (IP, user agent, version, target, arch, channel, install-ID header) and the retention/logging rules (no Workers Logs of request headers; head sampling; no Logpush of headers).
- `apps/updates/` (pnpm package `@lamina/updates`, private): Worker in TypeScript, `wrangler.jsonc` (pinned Wrangler as in C4), compatibility date pinned, routes `updates.<domain>/*`, R2 bindings `RELEASES` (bucket `lamina-releases`: stable, beta), `STAGING` (bucket `lamina-staging`: only the `staging` channel's manifest and test artefacts, so staging tests never hold a token that can write user-facing releases — D5b, D6b) and `NIGHTLY` (bucket `lamina-nightly`: the nightly channel's manifest and artefacts, written by the reviewer-free `nightly` CI environment with a token scoped to that bucket only, so it can never overwrite release files — D6a/D6b), optional KV not used (R2 + Cache API suffice), `observability` configured so request headers are **not** logged.
  - Routes: `GET /{channel}/{target}/{arch}/{current_version}` → 204/200; `GET /healthz`; everything else 404. `HEAD` supported. Strict parameter validation (`channel` allow-list, `target`/`arch` allow-lists, `current_version` semver) → 400 for malformed.
  - Manifest read from R2 `channels/{channel}.json` and cached in **one** layer only: the Cache API, with a **30 s** TTL (`Cache-Control: max-age=30` on the cached copy; a synthetic cache key per channel). No isolate-level memo — two stacked 60 s layers could serve a stale manifest for ~120 s and break the 60 s kill-switch target. Worst-case staleness = 30 s TTL + R2 read (strongly consistent), well inside 60 s. Note the Cache API is a no-op on `workers.dev` — staging tests of the TTL use the custom-domain staging route or fake timers. `paused` honoured for `release`/`previous`; bridges are evaluated first and ignore it.
  - Response: if the eligible release's version > `current_version` and the platform key `{target}-{arch}` exists → 200 `{ version, pub_date, url, signature, notes }`; else 204. `Cache-Control: no-store` on responses (they depend on the header).
  - Previous-release fallback: if the client is not in the rollout cohort of `release` but `previous` (100 %) is newer than `current_version`, serve `previous`.
  - Bridge releases: each entry is `{ "below": "0.5.0", "release": { …same shape as `release`: version, pub_date, notes, platforms with `url` + `signature` per `{target}-{arch}`… } }` (`rollout_percent` not allowed — bridges always apply to everyone below the bound). A client with `current_version < below` gets the bridge's platform entry; bridges take priority over `release`/`previous` (key rotation, breaking migrations). Validation rejects a bridge with no platforms, a platform without `url` or `signature`, or `release.version < below`.
- R2 layout (documented in ADR 0014 and `apps/updates/README.md`):
  - `releases/{version}/{target}-{arch}/{file}` and `{file}.sig` (e.g. `Lamina_0.2.0_aarch64.app.tar.gz`, `Lamina_0.2.0_x64-setup.exe`, `Lamina_0.2.0_x64_en-US.msi`, `Lamina_0.2.0_amd64.AppImage`), plus installers for download (`.dmg`, `.deb`, `.rpm`) and `SHA256SUMS`;
  - `channels/{channel}.json` manifests;
  - the same layout in `lamina-staging` for the `staging` channel (served through `staging-downloads.<domain>`) and in `lamina-nightly` for the nightly channel, served through its own custom domain (e.g. `nightly-downloads.<domain>` — one label deep, so Universal SSL covers it);
  - served publicly through the R2 custom domain `downloads.<domain>` (bucket public access only via the custom domain; `r2.dev` disabled).
- Manifest schema `apps/updates/src/manifest.ts` (validated with a small hand-written validator or `valibot`/`zod` — licence check) plus a JSON Schema file for CI tooling:
  ```json
  {
    "schema": 1,
    "channel": "beta",
    "paused": false,
    "release": {
      "version": "0.2.0",
      "pub_date": "2027-01-15T10:00:00Z",
      "notes": "https://<domain>/desktop/releases/0.2.0",
      "rollout_percent": 10,
      "platforms": {
        "darwin-aarch64": { "url": "https://downloads.<domain>/releases/0.2.0/darwin-aarch64/Lamina.app.tar.gz", "signature": "<.sig contents>" }
      }
    },
    "previous": { "version": "0.1.3", "...": "same shape, rollout_percent must be 100" },
    "bridges": [
      {
        "below": "0.5.0",
        "release": {
          "version": "0.5.0",
          "pub_date": "2027-06-01T10:00:00Z",
          "notes": "https://<domain>/desktop/releases/0.5.0",
          "platforms": {
            "darwin-aarch64": { "url": "https://downloads.<domain>/releases/0.5.0/darwin-aarch64/Lamina.app.tar.gz", "signature": "<.sig contents>" }
          }
        }
      }
    ]
  }
  ```
- `apps/updates/scripts/` (or `cargo xtask updates …`): `manifest set-rollout <channel> <percent>`, `manifest pause|resume <channel>`, `manifest promote <version> --to <channel>` writing to R2 via `wrangler r2 object put` — used by humans and by D6b's workflow.
- Deploy workflow for the Worker (same pattern as `apps/web` in C4: preview on PR if applicable, production from `main`), with `CLOUDFLARE_API_TOKEN` scoped to Workers + the one R2 bucket.

**Out of scope** (and which todo covers it)

- App-side updater wiring, settings, install-ID generation, managed policy reading, end-to-end update test: [D5b](D5b-updater-in-app.md).
- Producing signed artefacts and uploading them: [D6b](D6b-release-workflow-and-linux-packages.md)–[D6d](D6d-windows-signing.md).
- Crash-report endpoint on the same Worker: [D7a](D7a-crash-reporting.md).
- Download page on the website: [D7b](D7b-settings-renderer-switch-and-download-page.md).

## Tasks

- [ ] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2`, `workers-best-practices` and `wrangler` skills.
- [ ] Write ADR 0014 (template in `docs/decisions/README.md`); index it; update the architecture §3 table row.
- [ ] Scaffold `apps/updates` (package.json, `tsconfig.json` extending the base, Biome, Vitest with `@cloudflare/vitest-pool-workers` — licence check), `wrangler.jsonc` with `r2_buckets`, `routes`, `compatibility_date`, `observability: { enabled: true, head_sampling_rate: … }` and no header logging.
- [ ] `src/index.ts` (fetch handler), `src/route.ts` (parse/validate), `src/manifest.ts` (schema + validation), `src/rollout.ts` (`bucketFor(installId, version): Promise<number>` with `crypto.subtle.digest`), `src/semver.ts` (compare incl. prerelease like `0.1.1-nightly.20261028.42`; or a vetted small dependency), `src/cache.ts` (single Cache API layer, 30 s TTL).
- [ ] Header handling: read `Lamina-Install-Id`, validate UUID v4 (case-insensitive), lower-case before hashing; never pass it to `console.*` or include it in errors; strip it before any subrequest.
- [ ] Manifest tooling commands; JSON Schema; example manifests under `apps/updates/test/fixtures/`.
- [ ] **Owner action — updater key ceremony** (per ADR 0014 §7, before D5b): on an offline or freshly booted trusted machine run `pnpm tauri signer generate -w lamina-updater.key` with a strong password; store the private key + password in the secret manager and an encrypted offline backup in two places; add them as `release`-environment secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; repeat for the **nightly** key (`lamina-updater-nightly.key`, stored as `nightly`-environment secrets with the same names; one backup is enough — losing it only strands nightly installs, which can reinstall); give both **public** keys to the D5b author. Record in the PR only that it was done and the public key's fingerprint — never the key.
- [ ] Owner actions listed in the PR: create the R2 buckets `lamina-releases`, `lamina-staging` and `lamina-nightly`, connect `downloads.<domain>`, `staging-downloads.<domain>` and `nightly-downloads.<domain>` as their custom domains, create the `updates.<domain>` route, create three scoped API tokens (one per bucket) and store them as GitHub secrets: `lamina-releases` in `release`, `lamina-nightly` in `nightly`, and `lamina-staging` (`CLOUDFLARE_STAGING_R2_TOKEN`) in a `staging` environment (deployment branches: `main`) and also in `release`, for the signed staging dispatch runs of D6b–D6d (D6a). Until the domain exists, use a staging zone/`workers.dev` for tests (ADR 0014 notes it).
- [ ] Deploy workflow + staging environment (`staging` channel) for D5b's e2e test.

## Acceptance criteria

- [ ] ADR 0014 is **Accepted** and covers items 1–9 above, including the key rotation runbook and compromise procedure.
- [ ] Worker unit tests cover: 204 when up to date; 200 with the exact Tauri dynamic JSON when newer; 204 when the platform key is missing; rollout bucketing with the header **present** (in cohort / not in cohort, deterministic for a fixed ID+version), **absent** (only 100 % releases) and **malformed** (treated as absent); `previous` fallback; bridges (served below the bound with the bridge's own `url` + `signature`; not served at or above it; invalid bridge entries rejected by the validator); `paused` kill switch → 204 for clients outside every bridge, while a client below a bridge's `below` still gets the bridge with `paused: true`; manifest cache expiry: a `paused` change in R2 is visible after ≤ 30 s and there is no second cache layer (fake timers: request at t=0, flip `paused` at t=1 s, assert 204 by t=31 s); 400 on malformed path segments; unknown channel → 404.
- [ ] A test asserts the install ID never reaches logs: spies on every `console` method and on the observability/tail output across all test requests and fails if the ID (or its hash) appears.
- [ ] Rollout distribution test: 100,000 random UUIDs at 10 % → 10 % ± 0.5 %.
- [ ] Deployed to staging; `curl -H 'Lamina-Install-Id: <uuid>' https://<staging>/staging/darwin/aarch64/0.0.1` returns the expected JSON for a seeded manifest (recorded in the PR).
- [ ] `cargo xtask check ts` covers `apps/updates` (typecheck, Biome, Vitest).

## Post-merge checks

- [ ] The production deploy from `main` succeeds and `GET https://updates.<domain>/healthz` returns 200 (or the staging route until the domain exists — record which).
- [ ] A request for an unseeded channel/version on production returns 204 and no request header appears in Workers Logs (spot check recorded).

## Tests

- Vitest in the Workers runtime (`@cloudflare/vitest-pool-workers`) with Miniflare R2 seeded from `test/fixtures/*.json`.
- Property-style test for `bucketFor` (stable, uniform) and semver comparison table tests (prerelease ordering).
- Log-leak test described above.

## Docs to update

- `docs/decisions/0014-desktop-distribution.md` (new), `docs/decisions/README.md`, `docs/plans/architecture.md` §3 row.
- `docs/plans/desktop-app.md` §9 D5: tick "Write ADR 0014…", "Create `apps/updates` Worker…", "Implement rollout bucketing…", "Define the R2 layout and channel manifest schema; tests for every branch".
- `CLAUDE.md` layout (`apps/updates`) and commands; `apps/updates/README.md` (operations: promote, rollout, pause).

## Risks and open questions

- **Domain not chosen yet** (web plan §8 Q1): use placeholders `updates.<domain>`/`downloads.<domain>` in code via `vars`; the updater endpoint URL is compiled into the app config (D5b), so the domain must be fixed before the first public release (D7c) — changing it later needs a bridge release.
- **Cloudflare logging of headers**: Workers Logs/Tail/Logpush may capture request headers by default; verify which products record headers and disable or redact; record the verification in ADR 0014.
- **Cache vs kill switch**: R2 reads are strongly consistent but the Cache API is per data centre; the single 30 s TTL bounds staleness at 30 s per data centre — test the bound. Do not add an in-isolate memo later without subtracting its TTL from the cache TTL (sum must stay ≤ 60 s).
- **R2 write rate** (1 write/s per key) is irrelevant for manifests but the tooling must not loop-write.
- **Default channel during 0.x** (open question 4) — the owner decides; recorded in ADR 0014.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
