# 0014 — Desktop distribution: channels, staged rollout, the update Worker, signing identities and key custody

- Status: Accepted
- Date: 2026-11-12
- Amended 2026-11-15 (D7a): §10 adds crash reports, which the same Worker receives ([desktop plan §8 Q2](../plans/desktop-app.md#8-open-questions)).
- Amended 2026-11-15 ([D6c](../todos/done/D6c-macos-signing-and-notarisation.md)): §6 records how macOS builds are signed: one build per architecture, the hardened runtime only with the Developer ID, PDFium signed before Tauri bundles, the `.dmg` notarised on its own, and macOS 13 as the oldest supported version.
- Amended 2026-11-15: §7's updater keys replaced before anything published, because the first release key's password didn't unlock it.

## Context

The desktop reader ([ADR 0008](0008-desktop-shell.md)) installs outside any app store. After installation, only one path can change an installed copy: Tauri's updater, which installs a bundle only if the updater key signed it ([Tauri updater plugin](https://v2.tauri.app/plugin/updater/)). That path needs a service that answers each install, a staged release with a way to stop it fast, operating-system signatures, and safe custody of the keys behind them. Losing the updater key strands every installed copy, and leaking it lets anyone push code to them. The [desktop plan §5](../plans/desktop-app.md#5-design-summary) sketches the design, and the owner decided on 2026-10-28 to sign as the publisher personally ([architecture §8 Q5](../plans/architecture.md#8-open-questions-for-the-team)). This ADR records the service, its rules and the keys. The update Worker in `apps/updates` implements the service side (§2, §4, §5 and §9, and §3's handling of the ID); the desktop app creates the install ID and reads the managed policies of §1.

## Decision

In short: one Worker reads a manifest per channel from R2 and answers the updater. The channels are stable, beta and nightly. Releases roll out in stages by a hashed anonymous install ID, and a pause or a bridge release reaches every install within 60 seconds. The owner is the publisher on every operating system. Releases and nightlies have separate updater keys, and no request header reaches any log.

### 1. Channels

- **`stable`**, **`beta`** and **`nightly`** are the channels users can follow. **`staging`** exists for tests (D5b, D6b–D6d) and is never offered in the app.
- **An install follows the channel of the build it came from** (owner, 2026-11-12). A beta download stays on beta and a stable download stays on stable. The download page offers stable by default once a stable release exists, and Settings lets the user switch between stable and beta. Until the first stable release, the downloads are beta builds.
- **Nightly is a separate install line.** Nightly builds embed the nightly updater key (§7), so they accept only nightly-signed bundles. Moving between nightly and beta or stable means reinstalling (D5b).
- **Managed installs can pin a channel or turn updates off**, from a macOS configuration profile, the Windows registry (`HKLM\Software\Policies\Lamina\UpdateChannel` and `DisableUpdates`) or `/etc/lamina/policy.json` on Linux. D5b implements the reading.

### 2. Staged rollout

Each release in a manifest carries `rollout_percent`, from 0 to 100 in steps of 0.01. An install's bucket for a release is

```
bucket = u32_be(SHA-256(install_id_lowercase ‖ ":" ‖ release_version)[0..4]) mod 10 000
```

and the install gets the release when `bucket < rollout_percent × 100`. Hashing with the version gives each release its own cohorts, so the same installs aren't always first. A request without a valid install ID (§3) gets only releases at 100 %. The usual steps for stable are 1 %, 10 %, 50 % and 100 %. A test of 100 000 random IDs at 10 % lands between 9.5 % and 10.5 %.

### 3. The install ID

- A random UUID v4, created on first run and stored only in the app's config. Settings can reset it.
- The app sends it only to the update endpoint, as the `Lamina-Install-Id` request header, set with the updater plugin's `UpdaterBuilder::header` (the updater URL substitutes only `{{target}}`, `{{arch}}` and `{{current_version}}`). It never goes to the crash endpoint (D7a) or anywhere else.
- The Worker accepts it only as a UUID v4 in either case, lower-cases it, hashes it for §2 and drops it. It never logs it, never puts it in an error and never sends it on: the Worker makes no subrequests, and it reads R2 through its binding. A malformed header counts as absent.
- `apps/updates/test/log-leak.test.ts` watches every `console` method across every path through the handler, and `test/workerd.test.ts` watches workerd's own log output. Both fail if the ID, or a hash of it, appears.

### 4. The kill switch and bridges

- **Pause.** A manifest's `paused: true` makes the Worker answer 204 to every install that no bridge covers. `rollout_percent: 0` stops one release the same way.
- **Bridges.** A bridge is `{ "below": "X.Y.Z", "release": { … } }`, served first to every install older than `below`, to all of them at once, **even while the channel is paused**. A bridge must list at least one platform, each with a `url` and a `signature`, and its version must be at least `below`. An install several bridges behind takes the one with the lowest bound first. Bridges carry key rotations (§7) and breaking migrations, and rolling back a bad release means publishing a bridge with a higher version.
- **How fast.** Every change reaches every data centre within 60 seconds. The Worker keeps one cache layer: the Cache API, holding a manifest for 30 seconds. It checks the copy's age itself, so the bound holds even where the cache keeps an entry longer. R2 reads are strongly consistent. Worst case: 30 s of cache plus one read, well inside the 60 s budget. The margin is spare, not room for a second layer: **no second layer** (in-isolate memo, KV copy) may be added unless both layers together stay within 60 s.

### 5. The R2 layout, the manifest and the endpoint

| Bucket | Channels | Served from | Written by |
| --- | --- | --- | --- |
| `lamina-pdf-releases` | stable, beta | `downloads.laminapdf.com` | `desktop-release` jobs, after the owner approves |
| `lamina-pdf-nightly` | nightly | `nightly-downloads.laminapdf.com` | `desktop-nightly` jobs |
| `lamina-pdf-staging` | staging | `staging-downloads.laminapdf.com` | D5b's tests and signed trial runs of D6b–D6d |

- **Inside each bucket:** `releases/{version}/{target}-{arch}/{file}` and `{file}.sig` (for example `Lamina.PDF_0.2.0_x64-setup.exe`), the installers for download (`.dmg`, `.deb`, `.rpm`), `SHA256SUMS`, and `channels/{channel}.json`.
- **Public access** goes only through the custom domains, one label deep so Universal SSL covers them. The `r2.dev` address stays off. Each bucket has its own write token, so a token for one channel can't overwrite another's files.
- **The manifest:** `schema: 1`, `channel`, `paused`, and optional `release` (with `rollout_percent`), `previous` (at 100 %, older than `release`) and `bridges`. Each release has `version` (SemVer, nightlies such as `0.1.1-nightly.20261028.42`), `pub_date` (RFC 3339), `notes` and `platforms`, keyed by `{target}-{arch}` with Tauri's values. `apps/updates/src/manifest.ts` validates it, and `apps/updates/schemas/manifest.schema.json` describes it for other tools. The Worker refuses an invalid manifest by answering 204 and logging the reasons. The manifest tool refuses to write one.
- **The endpoint:** `GET|HEAD https://updates.laminapdf.com/{channel}/{target}/{arch}/{current_version}`.
  - **200** with `{ version, pub_date, url, signature, notes }` when an update applies.
  - **204** otherwise, including for a channel with no manifest yet.
  - **400** for an unknown target or arch, or a version that isn't SemVer.
  - **404** for an unknown channel or any other path. `/healthz` answers 200.
  - Every answer carries `Cache-Control: no-store`, because it depends on the header, and `X-Content-Type-Options: nosniff`, with a `Content-Type` on any answer that has a body. The Tauri updater reaches the endpoint with native HTTP, not from a page, so CORS and [ADR 0011 §4](0011-cloudflare-topology-and-privacy.md)'s page headers (CSP, frame and permissions policies) don't apply.
- **Selection order:** bridges, then (unless paused) `release` for installs in its rollout, then `previous`.
- **Deploys.** Every change to the Worker that reaches `main` deploys to staging, then to production in its own `updates-production` GitHub environment (apart from the web reader's `production`, which may gain reviewers or a tag rule), which rolls back automatically when its smoke test fails. There is no release tag as the web reader has, because a deploy changes only how manifests are read: what any install is offered comes from the manifests in R2. The deploy token can edit Workers scripts and routes and has no R2 access; only each bucket's own token, held by the GitHub environment that publishes to it, can write releases.

### 6. Signing identities

The owner, **Laurence Ininda**, is the publisher on every operating system. The setup, the secrets and their renewal are in the desktop release runbook, `docs/guidelines/desktop-release.md`, which [D6a](../todos/done/D6a-signing-runbook-and-owner-setup.md) adds.

| OS | Identity | Shown as |
| --- | --- | --- |
| macOS | Apple Developer ID Application certificate, individual membership, Team ID `35KBR7UHPZ`, notarised with an App Store Connect API key | Developer ID Application: Laurence Ininda (35KBR7UHPZ) |
| Windows | Azure Artifact Signing, Public Trust, individual identity validation (completed 2026-11-12); account `LaurenceIninda` (East US), certificate profile `LaminaPdfPublicTrust`; CI signs through an Entra app with a GitHub OIDC federated credential for `desktop-release` and no client secret | Laurence Ininda (CN and O), Brooklyn, NY, US |
| Linux | GPG key: certify-only primary `FA00 7EB8 6046 1729 9F59 6FC5 B9FC 925D 5E53 7171` (expires 2031-11-11), signing subkey `DD4D 8CE5 7C00 8BF6 05D4 73B6 7C7B 196F 2CB0 87F5` (expires 2028-11-11) for the AppImage, `.deb`, `.rpm` and `SHA256SUMS` | Laurence Ininda (Lamina PDF releases) \<releases@laminapdf.com\> |

- **Windows eligibility.** Artifact Signing accepts individuals only in the US and Canada ([quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart), checked 2026-11-12, updated 2026-10-08). The owner lives in the US, so no OV certificate is needed.
- **SmartScreen.** Microsoft gives no certificate instant reputation, Artifact Signing included ([SmartScreen reputation](https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation), checked 2026-11-12). The first downloads may show a warning that names the publisher.
- **Nightlies are not OS-signed** (D6b). macOS builds are ad-hoc signed, Windows builds unsigned, and Linux packages ship with `SHA256SUMS` only. A compromised nightly job therefore can't produce anything signed with the owner's identity.
- **macOS signing** (D6c). Release builds sign every piece of code with the Developer ID and give executables the hardened runtime:
  - **Per architecture.** Each architecture is built, signed and notarised on its own runner; there is no universal build.
  - **PDFium.** Tauri signs the app and the engine but not the files in `Resources/`, so `cargo xtask desktop-sidecar` signs PDFium's library first: the hardened runtime's library validation loads only libraries with the engine's own Team ID.
  - **No entitlements.** Neither the app nor the engine has an entitlement or a hardened-runtime exception: WKWebView needs none in a Developer ID app, and the engine's sandbox is its own Seatbelt profile ([ADR 0008 §5](0008-desktop-shell.md#5-sandboxing-per-os)). Tauri signs the engine with the app's entitlements, so an entitlement added to the app would reach the engine; the release workflow fails a build in which either has one.
  - **Notarisation.** Tauri notarises and staples the app. The release workflow notarises and staples the `.dmg`, which needs a ticket of its own for Gatekeeper's offline check.
  - **Ad-hoc builds** leave out the hardened runtime: an ad-hoc signature has no Team ID, so library validation would refuse PDFium and the engine would draw nothing.
  - **macOS 13 and later** (`bundle.macOS.minimumSystemVersion`).

### 7. The updater keys: custody and rotation

An updater key is the one key whose loss a new certificate can't repair. The owner generated two on 2026-11-12, a release key and a nightly key, with `pnpm tauri signer generate`, each with a strong password:

| Key | minisign key ID | Signs | GitHub environment |
| --- | --- | --- | --- |
| Release | `34929FE857511973` | stable and beta | `desktop-release` (the owner approves every run) |
| Nightly | `46010ED395B59CAF` | nightly only | `desktop-nightly` (no reviewer, `main` only) |

Their Tauri `pubkey` values, which D5b compiles into the apps' updater configuration, are in [`apps/updates/README.md`](../../apps/updates/README.md#updater-public-keys).

- **Custody.** Each private key and its password are in the owner's password manager. The release key also has an encrypted offline backup in two physical places. The nightly key has one backup, since losing it strands only nightly installs, which can reinstall. CI gets each key only as that environment's `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, never as a repository secret. The nightly environment holds no release key, so a leaked nightly key can't push an update to beta or stable installs.
- **Planned rotation.**
  1. Ship release N, signed with the old key, whose configuration embeds the new public key.
  2. Sign every later release with the new key.
  3. Keep N in each channel's manifest as a bridge with `below: N`, indefinitely, so an install that skipped N still takes it first. The nightly key rotates the same way.
- **Compromise.**
  1. Pause the affected channels (§4). That stops normal releases signed with the leaked key; bridges still flow.
  2. Publish a bridge whose `below` is above every version in the field, so every install takes it. Installed copies trust only the old key, so the bridge is signed with the old key and embeds the new public key; later releases use the new key.
  3. Revoke the leaked key: delete it from the environment and the backups.
  4. Tell users in the release notes, on the download page and in a GitHub security advisory.
- **Signed versions.** The apps set the updater's `requireSignedVersion` (D5b): an update is refused unless its signature's trusted comment names the version the endpoint announced, so a server or a manifest can't pair a newer version number with an older signed release. Every release is therefore signed with a Tauri CLI that records the version (D6b).
- **The risk the owner accepts:** until every install has taken the bridge, whoever holds the leaked key can also sign updates those installs accept. No updater design avoids this without a second key the app already trusts. Tauri accepts only one updater public key per build.

### 8. Attestations and SBOMs

Every release artefact carries a GitHub artifact attestation (ADR 0017's practice), and each release publishes CycloneDX SBOMs and the app's third-party notices ([ADR 0019](0019-supply-chain-and-licence-hygiene.md)). D6b builds them. GitHub makes build-provenance attestations only for public repositories, and this repository is public.

### 9. Privacy

- **What the endpoint sees:** the IP address, the user agent, the channel, target, arch and current version (in the path) and the install ID header. Nothing else is sent, and the response depends only on these.
- **What is kept:** nothing about a request. The Worker's [`wrangler.jsonc`](../../apps/updates/wrangler.jsonc) sets `observability.logs.invocation_logs: false`, so Workers Logs record only what the Worker writes with `console`. That is two events, each naming the channel: `manifest-invalid` with the validator's reasons, and `manifest-unavailable` with R2's error name and message. Neither can hold a request header. `observability.traces.enabled: false` keeps traces off. Cloudflare's traces record the user agent, the full URL and some request headers on every request ([spans and attributes](https://developers.cloudflare.com/workers/observability/traces/spans-and-attributes/), checked 2026-11-12). No Tail Worker is attached; Tail Workers receive request headers ([Tail Workers](https://developers.cloudflare.com/workers/observability/logs/tail-workers/)). No Logpush job reads this Worker.
- **Verified 2026-11-12** against [Workers Logs](https://developers.cloudflare.com/workers/observability/logs/workers-logs/): invocation logs hold the request's metadata, and `observability.logs.invocation_logs: false` turns them off. Anyone adding tracing, a Tail Worker or Logpush must first show that the install ID can't reach it, and must amend this section.
- **The privacy page** (D7b) says the same in plain words.

### 10. Crash reports

The app reports crashes to the update Worker, not to Sentry or another service, and only with the user's consent (amended 2026-11-15, D7a). A report holds what went wrong in the app and never the user's documents, their names or their folders.

- **Built on the computer, from an allow-list.** The app turns a crash into a report itself. A report holds the panic's message and source location, the panicking thread's frames (function names and offsets) and the crash kind. It also holds the version, channel, OS and its version, architecture, process kind, renderer, sandbox layers, a coarse uptime and a random report ID. It has no other field, so nothing else can be sent. A scrubber removes paths, `file:` URLs, document file names, e-mail addresses and long numbers from every free-text field. A webview report names only the error's type from a fixed list, since a script's message can quote document text; its frames keep only bundle files (`assets/index-3f2a.js`). The schema is `apps/updates/schemas/crash-report.schema.json`.
- **Consent.** Sending is off until the user turns it on. After the first crash the app asks once: "Send report" sends that report, and with "Always send reports after a crash" checked it sends every later one too. Settings › Privacy turns sending on and off. Held reports stay on the computer for 30 days at most, ten at a time.
- **No install ID.** The crash client never sends `Lamina-Install-Id`, and the Worker refuses a report that carries it. Each report gets a random ID of its own, kept nowhere else. The Worker still sees the sender's IP address, which it uses as the rate limit's key and doesn't keep.
- **The Worker.** `POST /crash` checks the body against the schema and refuses a free-text field that still looks like a path, a `file:` URL, an e-mail address or a long number. It caps a body at 64 KiB and allows ten reports a minute from one address. It stores each report in the private bucket `lamina-pdf-crash-reports` under `crash-reports/{yyyy}/{mm}/{dd}/{report_id}.json`, and a lifecycle rule deletes reports after 30 days. It logs no body and no address.
- **Native crashes** (D7d, 2026-11-15). A segmentation fault or access violation leaves no panic, so the app captures a minidump out of process, and nothing hostile is parsed in the main process.
  - **Capture.** A monitor process, the app's binary started with `--crash-monitor` outside the sandbox, hosts two minidump servers, one for the main process and one for its engines. Each process installs a crash handler that asks its server for a dump; the engine does so before its sandbox locks down. A hostile message can crash only the monitor, which the main process restarts at most 5 times a session, giving each new engine the current socket. Inside the monitor, a server loop that a bad client keeps breaking restarts at most 16 times a minute; then the monitor exits and the app replaces it.
  - **Limits.** Each server refuses a new dump while its folder holds 4, or after 6 in the last minute, so an engine can't flood the disk or the reader. It writes each dump under a temporary name and renames it once complete, so no dump is read half written. On Linux the engine's seccomp lets the crash handler name only the monitor's pid in `PR_SET_PTRACER`; without that pid there is no capture.
  - **Reading, sandboxed.** The main process reads a dump's bytes and deletes the file, whether what follows succeeds or fails, and refuses a dump over 64 MiB. A fresh engine in `--summarize-dump` mode, under the full engine sandbox, reads the bytes from stdin and writes an allow-listed summary to stdout. The app ends it after 10 s.
  - **What the report holds.** Only the `native` object joins the report: the crash reason, at most 64 frames of the crashing thread as a module's file name and an offset within it, and each module's file name and debug ID. It holds no function names, since a name read from memory could carry document text; `cargo xtask crash symbolicate` adds them for maintainers from the archived symbols. The main process's own native crashes go to the monitor's main folder, and the next launch reads the newest few into held reports, as it does a main-process panic.
  - **Windows** (D7f, 2026-11-16). `minidumper` 0.11's Windows server took the pid a client sent and opened that process to dump it, so a compromised engine could have had any of the user's processes dumped. The workspace carries `minidumper` with a patch (`third_party/minidumper`, offered upstream): the server asks the kernel for the connection's peer (`SIO_AF_UNIX_GETPEERPID`) and refuses a request naming any other process before it opens one, as its Linux server checks peer credentials. Capture then runs as on Unix. The engine connects while its start-up impersonation still holds the user's token, before the sandbox locks down, and the connection stays usable after.
  - **Symbols.** The release workflow archives the builds' Breakpad symbols in a private bucket, `lamina-pdf-symbols`.

## Consequences

- One Worker and three buckets serve every desktop update. Costs are R2 storage and reads; the Worker reads each channel's manifest at most once per data centre every 30 s.
- Releasing becomes data rather than a deploy: D6b's workflow uploads bundles and edits a manifest with `apps/updates/scripts/manifest.ts`, and the owner moves a rollout or pauses a channel with the same tool.
- The updater URL and the release public key are compiled into every installed copy. Moving `updates.laminapdf.com`, or replacing the key, needs a bridge release.
- Every install now carries an ID. It is random, resettable and never stored server-side, but the privacy page must still describe it.
- The owner's legal name appears on every signed file and in every installer prompt. If the project incorporates, the publisher identities move to the company: a new Apple organisation membership, a new Artifact Signing organisation validation and a new GPG key. The updater keys stay, so installed copies keep updating.

## Alternatives considered

- **A static manifest per channel, read directly by the updater:** this can't stage a rollout or serve bridges, and every change would wait for CDN caches.
- **KV for the manifests:** KV is eventually consistent, with up to 60 s for a write to be seen, and that alone uses the kill switch's whole budget. R2 is strongly consistent.
- **An in-isolate memo in front of the Cache API:** it would stack a second TTL on the first and break the 60 s bound (§4).
- **A query parameter for the install ID:** the updater URL substitutes only three variables, and query strings end up in more logs than headers do.
- **Bucketing by IP address or user agent:** both change as a user moves, and both are personal data to keep.
- **One updater key for every channel:** a nightly job, which runs without approval, could then sign updates that stable installs accept.
- **Tauri's `version_comparator` for rollbacks:** it needs app support and runs the decision on the client; a bridge with a higher version does the same from the server.
- **An OV certificate in a cloud HSM for Windows:** this is needed only outside the US and Canada. Artifact Signing costs less and keeps the private key in Microsoft's HSM, with no key to manage. D6a's runbook keeps OV as the fallback.
- **Sentry for crash reports (§10):** it groups crashes and reads symbols for free, but its SDKs upload minidumps whole, with whatever memory they caught, unless configured with care, and every report would go to another company. Our own Worker keeps reports on our bucket, under the privacy rules the update service already follows, at no cost at the expected volume.
