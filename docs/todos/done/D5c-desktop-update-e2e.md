# D5c — The desktop update's end-to-end test: 0.0.1 updates to 0.0.2 on three OSes, a tampered bundle is refused

| Field | Value |
| --- | --- |
| ID | `D5c` |
| Status | `done` |
| Plan | [Desktop app §6 D5](../../plans/desktop-app.md#d5-update-service) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D5b](./D5b-updater-in-app.md) |
| ADRs | Follows ADR 0014 (endpoint, channels, staging bucket), ADR 0008 (the `e2e` feature stays out of release builds) |
| Branch | `ci/d5c-desktop-update-e2e` |
| PR title | `ci(desktop): test the update from 0.0.1 to 0.0.2, and a tampered bundle, on three OSes` |
| Size | M (300–600, mostly the workflow and its scripts) |
| PR | [#637](https://github.com/laudebugs/laminapdf/pull/637) |

Split from D5b on the owner's call (2026-11-14): D5b wires the updater into the app with unit tests; this todo proves it end to end. D5 is accepted when this todo's checks pass.

## Goal

Show that an installed Lamina PDF `0.0.1` updates itself to `0.0.2` on macOS, Windows and Linux (AppImage) from the `staging` channel, and that a tampered bundle is refused and leaves the app on `0.0.1`.

## Context

- D5b's updater: `apps/desktop/src-tauri/src/update/` (the endpoint per channel, the `Lamina-Install-Id` header, Check now, install on quit and Restart to update), `requireSignedVersion` on, the release public key in `tauri.conf.json`.
- The Worker and the staging bucket: [`apps/updates/README.md`](../../../apps/updates/README.md) (`staging-updates.laminapdf.com`, `lamina-pdf-staging`, the manifest format and its tool).
- Tauri's updater artefacts: `bundle.createUpdaterArtifacts: true` makes `.app.tar.gz` (macOS), the NSIS `-setup.exe` (Windows) and the `.AppImage` (Linux), each with a `.sig`. Signing needs `TAURI_SIGNING_PRIVATE_KEY`, so the base `tauri.conf.json` leaves it off and this workflow passes it with `--config`.

## Scope

**In scope**

- `.github/workflows/desktop-update-e2e.yml`, in two modes:
  - **`local`:** path-filtered `pull_request` (forks included), with **no secret and no environment**. The Worker runs on the runner with `wrangler dev` and local R2, seeded with `wrangler r2 object put --local`.
  - **`staging`:** `push` to `main` (path-filtered), a nightly schedule and `workflow_dispatch`, never `pull_request`. It runs in a `desktop-staging` environment (deployment branches: `main` only) holding `CLOUDFLARE_STAGING_R2_TOKEN`, scoped to `lamina-pdf-staging` only, against the deployed staging Worker. No job gets a token for `lamina-pdf-releases`.
- **A throwaway updater key per run** (`pnpm tauri signer generate` in the job): both versions are built in the same run, so `0.0.1` embeds the run's public key (a `--config` override of `plugins.updater.pubkey`) and `0.0.2` is signed with its private key. No signing secret is stored, and pull requests need none. This replaces D5b's planned `TAURI_TEST_SIGNING_PRIVATE_KEY`.
- **Builds:** `0.0.1` and `0.0.2` for the three OSes with the `e2e` feature, which lets the updater reach `http://127.0.0.1:<port>` (`dangerousInsecureTransportProtocol`, `e2e` only; D2a's guard keeps it out of release builds) and adds a test-only flag that forces the `staging` channel and an immediate check. Plain `X.Y.Z` versions: WiX/MSI accepts only a numeric pre-release ≤ 65535, and versions below `0.1.0` are reserved for this test, never published to a user channel.
- **The run:** upload `0.0.2` to the bucket, seed `channels/staging.json`, install `0.0.1`, launch it with the flag, wait for the install and relaunch, and assert the running version is `0.0.2`. A second run serves `0.0.2` with one byte flipped and its original `.sig`, and asserts the update is refused (the signature error logged) and the app stays on `0.0.1`.
- **The header:** the staging run asserts the Worker saw `Lamina-Install-Id` and that the URL never held it (the local Worker's request log, or a staging-only check).
- **macOS quarantine:** the test bundles are ad-hoc signed; the workflow may need `xattr -dr com.apple.quarantine`.

**Out of scope**

- The release workflow and OS code signing: [D6b](../done/D6b-release-workflow-and-linux-packages.md)–[D6d](../ready/D6d-windows-signing.md).

## Acceptance criteria

- [x] On macOS, Windows and Linux (AppImage), an installed `0.0.1` updates to `0.0.2` from the `staging` channel with no input beyond what `installMode: passive` shows (the `local`-mode run green on all three OSes in the PR; run linked). [run 43717200595](https://github.com/laudebugs/laminapdf/actions/runs/43717200595): each OS reported `started 0.0.1`, `ready 0.0.2`, `started 0.0.2`.
- [x] A tampered bundle is refused on each OS: the signature error is logged and the app stays on `0.0.1` (same workflow). Each OS reported `failed: The signature verification failed`, and the next step started on `0.0.1`.
- [x] The workflow's `pull_request` jobs name no secret and no environment; only `staging`-mode jobs use `desktop-staging` (a `workflows.rs` test holds it): `the_update_tests_pull_request_job_should_hold_no_secret`, which also reads the local actions the job uses.
- [x] The update request carries `Lamina-Install-Id` as a header, never in the URL (asserted in the run): every request, 0.0.2's included, through the test's recording proxy.

## Post-merge checks

- [x] The owner has created the `desktop-staging` environment (deployment branches: `main`) with `CLOUDFLARE_STAGING_R2_TOKEN` scoped to `lamina-pdf-staging` only, and its `STAGING_R2_ACCESS_KEY_ID` (#649).
- [x] The first `staging`-mode run on `main` is green on all three OSes and actually ran: until the token exists, each leg passes with a notice that it skipped, which doesn't count (a new workflow can't be dispatched or scheduled until it is on `main`, and PRs get no secrets). The first real run, 43735229390, failed: a bucket-scoped token can't use the REST API Wrangler calls, so #650 moved the writes to R2's S3 API. [Run 43742007373](https://github.com/laudebugs/laminapdf/actions/runs/43742007373) then passed on all three OSes, through the staging bucket.

## Docs to update

- `docs/plans/desktop-app.md` §9 D5: tick "End-to-end update test on a staging channel on three OSes; tampered bundle rejected".
- `apps/desktop/README.md`: running the update test locally.

## Decisions and deviations

- **The bundles' addresses in local mode.** The Worker serves only `https` bundle addresses, which a runner can't serve. So the local run's manifest names `https://staging-downloads.laminapdf.com/e2e/…`, and the test's proxy, which the app asks, rewrites that origin to a file server on the runner. The Worker and its manifest checks are unchanged. Staging mode uploads the bundles to `lamina-pdf-staging` and rewrites nothing.
- **The header check.** A proxy in front of the Worker records each request, in both modes. The test asserts that each request asked `/staging/{target}/{arch}/0.0.1`, with a UUID v4 in `Lamina-Install-Id` and the ID nowhere in the URL. No log of the Worker is read, and the ID is never written to a file.
- **The test-only flag is a file.** `update-test.json` in the e2e build's config folder names the server and the report file; only `update/e2e.rs`, in `e2e` builds, reads it. The first run used environment variables, but on Windows the NSIS installer relaunches the updated app through the desktop shell as the signed-in user, so 0.0.2 started without them and never reported. A file in the config folder works however the app is launched. Release binaries are checked for `update-test.json` along with the other e2e names.
- **Plain HTTP** to the runner's servers comes only from the workflow's `--config` override (`dangerousInsecureTransportProtocol`). A capability-guard test fails if any committed `tauri*.conf.json` sets it.
- **One install, two steps.** The tampered bundle goes first, then the real one, so the second step also shows that a refused update leaves a working `0.0.1`. Each step waits until the Worker serves its manifest (it caches one for up to 30 s).
- **Staging legs run one at a time** (`max-parallel: 1`), since each writes the channel's one manifest. Each leg leaves the channel paused and deletes its bundles. Until `desktop-staging` holds `CLOUDFLARE_STAGING_R2_TOKEN`, the staging job passes with a notice.
- **The staging run goes through the proxy too**, over plain HTTP on the runner, so it can see the header. So the app's own TLS request to the deployed Worker isn't what's tested; the proxy's is. The app's HTTPS requests are D5b's, against production, and the release workflow's trial runs.
- **One lock for the staging channel:** staging runs take the concurrency group `desktop-staging-channel`, which the desktop release workflow's signed trial runs (D6b) should share, since both write `channels/staging.json`.
- **The hooks live in `src/`**, gated by the `e2e` feature, like `e2e.rs`'s commands: release builds compile `update/e2e_off.rs`, whose hooks do nothing, and CI checks their binary.
- **Debug builds** (`tauri build --debug`): the updater works the same way, and debug builds compile faster.

## Definition of done

- [x] Work PR from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` (Status `ongoing`)
- [x] `/code-review` process followed (findings and fixes recorded in commit messages)
- [x] CI green
- [x] This file moved to `done/` in the work PR (Status `done`), plan to-do ticked, board regenerated
