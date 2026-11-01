# D6b — Release workflow (tauri-action matrix → R2), Linux packages and signing, nightly channel

| Field | Value |
| --- | --- |
| ID | `D6b` |
| Status | `ready` |
| Plan | [Desktop app §6 D6](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D5b](./D5b-updater-in-app.md), [D6a](./D6a-signing-runbook-and-owner-setup.md) |
| Blocks | [D6c](./D6c-macos-signing-and-notarisation.md), [D6d](./D6d-windows-signing.md), [D7a](./D7a-crash-reporting.md), [D7b](./D7b-settings-renderer-switch-and-download-page.md) |
| ADRs | Implements ADR 0014 (R2 layout, manifests, attestations). Follows [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (pinned actions, SBOMs, third-party notices), ADR 0017 (attestations, library plan) |
| Branch | `ci/d6b-desktop-release-workflow` |
| PR title | `ci(desktop): add the tauri-action release matrix with Linux signing and R2 publishing` |
| Size | L (600–1,200 lines of workflow, scripts and config) |
| PR | — |

## Goal

One workflow turns a `desktop-v*` tag (and, nightly, `main`) into published desktop releases: it builds every target with `tauri-apps/tauri-action`, produces updater artefacts and installers, signs the Linux packages with the owner's GPG key (tagged releases), signs updater artefacts with the updater key (the release key for tags, a nightly-only key for nightlies), generates SBOMs, the third-party notices and GitHub artifact attestations, uploads everything to R2 under the ADR 0014 layout, and updates the channel manifest. macOS and Windows OS-level signing slot into this pipeline in D6c and D6d; until both have merged the workflow publishes **only to the `nightly` channel** (enforced in the workflow). The plan's "a nightly channel build publishes automatically" criterion is met here.

## Context

- [Desktop plan §6 D6 and §9 D6](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline): matrix macOS arm64/x64, Windows x64/arm64, Linux x64/arm64; AppImage (updater) + deb, rpm if cheap; R2 with `.sig` and attestations; manifests per channel.
- ADR 0014 (R2 layout, manifest schema, manifest tooling from D5a), `docs/guidelines/desktop-release.md` (D6a: secret names, verification commands).
- [Report: repository section](../../research/architecture/app-and-package-architecture.md#the-repository-is-cargo-pnpm-and-a-task-runner-released-through-oidc-and-gated-by-a-pdf-corpus): Release lane runs tauri-action; desktop binaries get GitHub attestations alongside Tauri's updater signature.
- [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md): actions pinned by SHA; CycloneDX SBOMs per artefact; generated third-party notices shipped inside the app.
- Tauri: [GitHub pipeline guide](https://v2.tauri.app/distribute/pipelines/github/) (`tauri-apps/tauri-action`, `args: --target …`, `projectPath`, `tauriScript`), [Linux distribution](https://v2.tauri.app/distribute/) (AppImage, deb, rpm; `bundle.linux.deb.depends`, `bundle.linux.rpm.depends`, `bundle.linux.appimage.bundleMediaFramework`), [Linux signing](https://v2.tauri.app/distribute/sign/linux/) (AppImage GPG signing via `SIGN=1`, `SIGN_KEY`, `APPIMAGETOOL_SIGN_PASSPHRASE`; check the rpm signing env vars in the current docs).
- [Desktop plan §4 non-functional](../../plans/desktop-app.md#non-functional-targets-and-release-gates): installer < 15 MB per platform (owner decision 2026-10-29; was ≤ 40 MB); supported OS list (Ubuntu 22.04+ → build Linux on 22.04 for glibc).

## Scope

**In scope**

- `.github/workflows/desktop-release.yml`:
  - Triggers: `push: tags: ['desktop-v*']`, `schedule` (nightly from `main`, only if there are new commits touching desktop paths), `workflow_dispatch` (channel input: **`staging` only** — beta and stable are published only from tags, and nightly only from the schedule), and `pull_request` (path-filtered) in forced dry-run mode only — never with an environment or secrets.
  - Job `plan` (no environment, no secrets): derive the **version**, the target channel, the **environment** (`release` for `desktop-v*` tags and `workflow_dispatch`, `nightly` for the schedule) and whether macOS/Windows signing is available (until D6c/D6d merge: nightly only). Version rules, chosen to satisfy both semver (the updater) and WiX/MSI (whose version is `major.minor.patch[.build]` with major/minor ≤ 255, patch/build ≤ 65,535, and which [Tauri's bundler](https://v2.tauri.app/reference/config/) only lets through a pre-release identifier that is numeric and ≤ 65,535):
    - **Tags**: `desktop-vX.Y.Z` → `X.Y.Z`, plain only. Tags with a pre-release or build suffix are rejected by `plan`, as are X or Y > 255 or Z > 65,535. Tagged builds produce **both** NSIS and MSI.
    - **Nightlies**: `X.Y.Z-nightly.YYYYMMDD.N`, where `X.Y.Z` is the next patch after the latest `desktop-v*` tag (so nightlies sort after the last release and before the next one); **before the first tag** `X.Y.Z` is `0.1.0`, the first release version (D7c), so pre-release nightlies sort below it and above the `0.0.*` staging range, `YYYYMMDD` is the UTC build date and `N` is `github.run_number`. Every numeric identifier is non-zero-leading by construction (the date starts with `2`, `run_number` ≥ 1); no commit SHA in the version (a 7-hex SHA can be all digits with a leading zero, which is invalid semver) — the SHA goes in the manifest `notes` and the job summary instead. Nightlies build **NSIS only** on Windows (`--bundles nsis`), because MSI can't carry a non-numeric pre-release; the updater uses the NSIS artefact anyway (per-user install, D6d).
    - **Staging (`workflow_dispatch`)**: `0.0.B` with `B = 100 + github.run_number` (monotonic per workflow, plain numeric so tagged-style NSIS **and** MSI are built; ≤ 65,535, which leaves 65,435 runs). The `0.0.*` range is reserved for staging and never published to a user channel (D5b uses `0.0.1`/`0.0.2` for its own test; `B` ≥ 101 never collides with them). Staging builds publish only to the separate `lamina-pdf-staging` bucket (D5a), so they can never land on a path of a real release, and that bucket's objects are uploaded with `Cache-Control: no-store` (test artefacts may be overwritten), unlike the immutable release paths.
    - `cargo xtask desktop-version` implements these rules (pure function + table tests) and the workflow fails if it returns an error.
  - Job `build` (environment from `plan`: `release` or `nightly`; matrix): `macos-15` (arm64; `aarch64-apple-darwin`), `macos-15-intel` (`x86_64-apple-darwin`; the old `macos-13` Intel label is retired — current Intel labels are `macos-15-intel`/`macos-26-intel`, check [GitHub's runner list](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) when pinning; cross-compiling from arm64 with `--target x86_64-apple-darwin` is the fallback) — or one `universal-apple-darwin` build if size allows (decide; record), `windows-2022` (`x86_64-pc-windows-msvc`), `windows-11-arm` or cross (`aarch64-pc-windows-msvc`), `ubuntu-22.04` (`x86_64-unknown-linux-gnu`), `ubuntu-22.04-arm` (`aarch64-unknown-linux-gnu`). Steps: checkout, toolchains (`rust-toolchain.toml` + target), pnpm, Linux deps, build the engine sidecar for the target (`cargo xtask desktop-sidecar --target …`), fetch pinned PDFium for the target, `tauri-apps/tauri-action@<sha>` with `args: --target <triple> --bundles <list>` and **no GitHub Release creation** (artefacts stay in the workflow), env `TAURI_SIGNING_PRIVATE_KEY`/`_PASSWORD` (updater — the environment decides which key: release or nightly), Linux signing env (release only). Every build passes `--config src-tauri/tauri.bundle.conf.json` (sidecar, PDFium and notices resources — D2b keeps them out of the base config so plain `cargo` builds work); nightly builds also pass `--config src-tauri/tauri.nightly.conf.json` (the CLI accepts `--config` more than once and merges in order — confirm with the pinned CLI; if not, `cargo xtask` merges both into one temp file), which sets the nightly updater public key and endpoint channel (D5b) and `bundle.windows`/targets overrides; it must not drop any array entries of the base config (JSON merge patch replaces arrays — repeat full arrays).
  - `bundle.targets` per OS in `tauri.conf.json`: macOS `["app", "dmg"]`, Windows `["nsis", "msi"]` (nightlies narrow this to NSIS with `--bundles nsis`), Linux `["appimage", "deb", "rpm"]` (rpm kept if the matrix stays green; else dropped with a note).
  - **Nightly signing policy (decided)**: nightlies run in the `nightly` environment, which has no required reviewer and holds only nightly-scoped secrets (D6a). They are **not OS-signed**: macOS ad-hoc signed (`codesign -s -`, needed on Apple silicon; Gatekeeper will warn on first launch — the nightly download page says so), Windows unsigned (SmartScreen warns), Linux packages unsigned with `SHA256SUMS` only. Their updater artefacts are signed with the **nightly-only updater key**, and nightly builds embed the matching public key, so nightlies only update to nightlies. Tagged releases keep the `release` environment and its reviewer gate, the production updater key and every OS signing identity.
  - Linux signing (tagged releases): AppImage GPG-signed (embedded signature); `.deb` signed with `debsigs --sign=origin` and verified with `debsig-verify` using a policy file shipped in the runbook; `.rpm` signed with `rpmsign --addsign` (or Tauri's rpm signing env vars if available) and verified with `rpm -K`; detached `.asc` for every Linux file; `SHA256SUMS` + `SHA256SUMS.asc`.
  - Job `sbom-and-notices`: CycloneDX SBOMs — Rust via `cargo-cyclonedx` (or `cargo-about`), npm via `pnpm` + syft over the installers; third-party notices generated with `cargo-about` (Rust, MIT/Apache) + `license-checker`-style output for npm, bundled into the app as a resource (`bundle.resources` in `tauri.bundle.conf.json`, as one array together with PDFium — JSON merge patch replaces arrays) and shown under Help ▸ View Licences (D4b). Notices generation runs **before** the build (it's packaged), SBOMs after.
  - Job `attest`: `actions/attest-build-provenance@<sha>` for every artefact (subject paths), plus SBOM attestations (`actions/attest-sbom`).
  - Job `publish` (same environment as `build`, needs all): upload to R2 `releases/{version}/{target}-{arch}/` — bucket `lamina-pdf-releases` for tagged releases, `lamina-pdf-staging` for `workflow_dispatch` staging runs, `lamina-pdf-nightly` for nightlies (ADR 0014; each token can write only its own bucket) — with `wrangler r2 object put` (or `aws s3 cp --endpoint-url` with R2 S3 credentials), set `Content-Type` and `Cache-Control: public, max-age=31536000, immutable` for versioned files; then update `channels/{channel}.json` via D5a's tooling with `rollout_percent` from input (nightly: 100; beta/stable: starts at the input value, default 1 per ADR 0014 for stable); verify the manifest by calling the Worker for each platform with a synthetic install ID; post a summary (versions, sizes, links) to the job summary.
  - Size budget step: fail if any installer reaches 15 MB (warn at 13.5 MB).
- `xtask` helpers if useful: `cargo xtask desktop-version` (sync version from tag into `apps/desktop/package.json`/`tauri.conf.json` at build time without committing).
- `docs/guidelines/desktop-release.md` gains a "Cutting a release" section (tag, watch, verify, promote, roll out, pause).
- Clean-VM verification for Linux (Ubuntu 22.04, Ubuntu 24.04, Fedora current): install the deb/rpm, run the AppImage, verify signatures with the runbook commands; recorded in this file after the first `release`-environment run (Post-merge checks).

**Out of scope** (and which todo covers it)

- Apple signing, notarisation and stapling: [D6c](D6c-macos-signing-and-notarisation.md) (this PR builds macOS unsigned/ad-hoc for nightly only).
- Windows Authenticode signing: [D6d](D6d-windows-signing.md) (this PR builds Windows unsigned for nightly only).
- Download page: [D7b](D7b-settings-renderer-switch-and-download-page.md). First beta: [D7c](D7c-desktop-beta-release.md).
- Flatpak/Snap/stores (non-goals).

## Tasks

- [ ] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2`; `wrangler` for the R2 steps.
- [ ] Pin every action by commit SHA with a version comment; add them to Dependabot's `github-actions` ecosystem.
- [ ] Write the workflow jobs above; reuse D5b's staging workflow pieces (factor shared steps into a composite action under `.github/actions/desktop-build/`).
- [ ] Configure `bundle.targets`, `bundle.linux.deb.depends` (e.g. `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libcups2` if D4c needs it), `bundle.linux.rpm.depends`, `bundle.linux.appimage.bundleMediaFramework: false`, desktop file template from D3a, `bundle.resources` for notices and PDFium in `tauri.bundle.conf.json` (D2b).
- [ ] Linux signing steps and verification; publish the debsig policy XML and keyring instructions in the runbook.
- [ ] SBOM, notices, attestations jobs.
- [ ] R2 publishing + manifest update + post-publish verification against the Worker.
- [ ] Nightly-only guard until D6c and D6d merge (a workflow input `allow_unsigned_platforms` that defaults to false for non-nightly channels).
- [ ] `tauri.nightly.conf.json` (nightly updater pubkey, NSIS-only Windows bundle) and the environment selection in `plan`; assert in a workflow step that a `nightly`-environment job never references `APPLE_*`, `AZURE_*`, `LINUX_SIGNING_*` secrets (grep of the workflow file in CI).
- [ ] `pull_request`-triggered dry-run mode (path-filtered to the workflow and `apps/desktop/**`) used to prove the workflow before merge; the end-to-end nightly and release runs are Post-merge checks.

## Acceptance criteria

The workflow is new, so `schedule` and `workflow_dispatch` can't run it until it is on `main`, and neither environment deploys from a PR branch; the PR proves it with a `pull_request`-triggered **dry run** (`dry_run: true`: no environment, no secrets, updater artefacts and OS/Linux signing skipped, no uploads). Everything that needs an environment or a published artefact is in Post-merge checks.

- [ ] The dry run builds all six target/arch combinations (or the documented subset if a runner is unavailable, with the gap recorded) with the configured bundles.
- [ ] Version derivation: nightly versions match `^\d+\.\d+\.\d+-nightly\.[1-9]\d{7}\.[1-9]\d*$`; staging versions match `^0\.0\.(10[1-9]|1[1-9]\d|[2-9]\d{2}|[1-9]\d{3,4})$` and stay ≤ 65,535; `workflow_dispatch` with a channel other than `staging` fails `plan`; a pre-release tag (e.g. `desktop-v0.2.0-rc.1`) fails `plan`; the dry run's tag-mode Windows build produces a valid MSI and NSIS installer and its nightly-mode Windows build produces NSIS only.
- [ ] `plan` selects the `nightly` environment for `schedule` and `release` for tags/`workflow_dispatch` (unit test of the selection script); the CI grep proves no `nightly`-environment job references `APPLE_*`, `AZURE_*` or `LINUX_SIGNING_*`.
- [ ] Every dry-run artefact has a CycloneDX SBOM; the attestation step is wired (it runs for real only with `id-token: write` on non-dry runs).
- [ ] The app bundles a third-party notices file and Help ▸ View Licences shows it.
- [ ] Installer sizes < 15 MB (budget step green; sizes in the job summary).
- [ ] Non-nightly channels cannot be published while macOS/Windows signing is missing (workflow guard tested in the dry run).

## Post-merge checks

- [ ] The first scheduled nightly on `main` runs in the `nightly` environment **without** waiting for approval, builds every target, uploads to `lamina-pdf-nightly`, updates `channels/nightly.json`, and the post-publish Worker check returns 200 with valid URLs and signatures for every published platform key (run linked in this file).
- [ ] Every published nightly artefact passes `gh attestation verify <file> --repo <owner>/<repo>`.
- [ ] A nightly-installed AppImage (and NSIS build) on a clean VM auto-updates to the next scheduled nightly (proves the published pipeline works with D5b's updater).
- [ ] The first `release`-environment run (a `workflow_dispatch` to `staging`, or D7c's `desktop-v0.1.0` tag) waits for the owner's approval before `build`, and on clean Ubuntu 22.04/24.04 and Fedora VMs its packages pass `debsig-verify` (or the documented equivalent), `rpm -K` and the AppImage signature check; the deb/rpm install and the app launches (recorded here).

## Tests

- Workflow dry runs (`pull_request` trigger with `dry_run: true`, skipping environments, signing and uploads) in the PR; `workflow_dispatch` dry runs after merge.
- Unit tests for any xtask/scripts: version derivation table (tag `desktop-v0.1.0` → `0.1.0`; tags with pre-release/build suffixes, major/minor > 255 or patch > 65,535 → error; nightly after `desktop-v0.1.0` on 2026-10-28 run 42 → `0.1.1-nightly.20261028.42`; nightly sorts above `0.1.0` and below `0.1.1`; no tag yet on 2026-10-28 run 42 → `0.1.0-nightly.20261028.42`, which sorts above `0.0.65535` and below `0.1.0`; staging run 7 → `0.0.107`; staging with run number > 65,435 → error; no identifier has a leading zero); manifest update.
- Clean-VM verification checklist (Linux) in the PR.

## Docs to update

- `docs/plans/desktop-app.md` §9 D6: tick "Release workflow with tauri-action matrix…", "Linux AppImage (updater) and deb; rpm if the matrix allows", "Upload to R2 with `.sig` files and GitHub attestations; update channel manifests" and the Linux part of "Verify installs on clean VMs".
- `docs/guidelines/desktop-release.md`: "Cutting a release".
- `CLAUDE.md`: release workflow mention; `apps/desktop/README.md`.

## Risks and open questions

- **arm64 runners** (Windows arm64, Linux arm64) availability/cost on GitHub; cross-compiling Tauri for Windows arm64 from x64 works with MSVC; Linux arm64 needs native or cross WebKitGTK — fall back to native arm64 runners or drop arm64 Linux for beta (record).
- **macOS universal vs per-arch**: universal doubles the sidecar/PDFium size and very likely breaks the 15 MB budget — measure, and ship per-arch dmgs if it does.
- **rpm signing support** in Tauri's bundler may require a post-processing step; verify current docs.
- **Nightly noise**: avoid publishing identical nightlies (skip if no desktop-relevant changes).
- **Secret exposure in PR workflows**: the release workflow must never run on `pull_request` from forks; the `release` environment's reviewer gate protects production secrets; the `nightly` environment is limited to `main` and holds nothing that can sign as the owner or reach beta/stable installs.
- **Nightly UX without OS signing**: Gatekeeper/SmartScreen warnings on every nightly install; acceptable for a testers-only channel — revisit if nightly users grow (would need a reviewer-free path to OS signing, which this design deliberately avoids).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
