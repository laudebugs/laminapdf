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

One workflow turns a `desktop-v*` tag (and, nightly, `main`) into published desktop releases: it builds every target with `tauri-apps/tauri-action`, produces updater artefacts and installers, signs the Linux packages with the owner's GPG key, signs updater artefacts with the updater key, generates SBOMs, the third-party notices and GitHub artifact attestations, uploads everything to R2 under the ADR 0014 layout, and updates the channel manifest. macOS and Windows OS-level signing slot into this pipeline in D6c and D6d; until both have merged the workflow publishes **only to the `nightly` channel** (enforced in the workflow). The plan's "a nightly channel build publishes automatically" criterion is met here.

## Context

- [Desktop plan §6 D6 and §9 D6](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline): matrix macOS arm64/x64, Windows x64/arm64, Linux x64/arm64; AppImage (updater) + deb, rpm if cheap; R2 with `.sig` and attestations; manifests per channel.
- ADR 0014 (R2 layout, manifest schema, manifest tooling from D5a), `docs/guidelines/desktop-release.md` (D6a: secret names, verification commands).
- [Report: repository section](../../research/architecture/app-and-package-architecture.md#the-repository-is-cargo-pnpm-and-a-task-runner-released-through-oidc-and-gated-by-a-pdf-corpus): Release lane runs tauri-action; desktop binaries get GitHub attestations alongside Tauri's updater signature.
- [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md): actions pinned by SHA; CycloneDX SBOMs per artefact; generated third-party notices shipped inside the app.
- Tauri: [GitHub pipeline guide](https://v2.tauri.app/distribute/pipelines/github/) (`tauri-apps/tauri-action`, `args: --target …`, `projectPath`, `tauriScript`), [Linux distribution](https://v2.tauri.app/distribute/) (AppImage, deb, rpm; `bundle.linux.deb.depends`, `bundle.linux.rpm.depends`, `bundle.linux.appimage.bundleMediaFramework`), [Linux signing](https://v2.tauri.app/distribute/sign/linux/) (AppImage GPG signing via `SIGN=1`, `SIGN_KEY`, `APPIMAGETOOL_SIGN_PASSPHRASE`; check the rpm signing env vars in the current docs).
- [Desktop plan §4 non-functional](../../plans/desktop-app.md#non-functional-targets-are-recommendations): installer ≤ 40 MB per platform; supported OS list (Ubuntu 22.04+ → build Linux on 22.04 for glibc).

## Scope

**In scope**

- `.github/workflows/desktop-release.yml`:
  - Triggers: `push: tags: ['desktop-v*']`, `schedule` (nightly from `main`, only if there are new commits touching desktop paths), `workflow_dispatch` (channel input).
  - Job `plan`: derive version (tag → `0.1.0`, nightly → `0.1.0-nightly.YYYYMMDD.<sha7>` semver-valid), target channel, and whether macOS/Windows signing is available (until D6c/D6d merge: nightly only).
  - Job `build` (environment `release`; matrix): `macos-14` (`aarch64-apple-darwin`), `macos-13` or cross (`x86_64-apple-darwin`) — or one `universal-apple-darwin` build if size allows (decide; record), `windows-2022` (`x86_64-pc-windows-msvc`), `windows-11-arm` or cross (`aarch64-pc-windows-msvc`), `ubuntu-22.04` (`x86_64-unknown-linux-gnu`), `ubuntu-22.04-arm` (`aarch64-unknown-linux-gnu`). Steps: checkout, toolchains (`rust-toolchain.toml` + target), pnpm, Linux deps, build the engine sidecar for the target (`cargo xtask desktop-sidecar --target …`), fetch pinned PDFium for the target, `tauri-apps/tauri-action@<sha>` with `args: --target <triple> --bundles <list>` and **no GitHub Release creation** (artefacts stay in the workflow), env `TAURI_SIGNING_PRIVATE_KEY`/`_PASSWORD` (updater), Linux signing env.
  - `bundle.targets` per OS in `tauri.conf.json`: macOS `["app", "dmg"]`, Windows `["nsis", "msi"]`, Linux `["appimage", "deb", "rpm"]` (rpm kept if the matrix stays green; else dropped with a note).
  - Linux signing: AppImage GPG-signed (embedded signature); `.deb` signed with `debsigs --sign=origin` and verified with `debsig-verify` using a policy file shipped in the runbook; `.rpm` signed with `rpmsign --addsign` (or Tauri's rpm signing env vars if available) and verified with `rpm -K`; detached `.asc` for every Linux file; `SHA256SUMS` + `SHA256SUMS.asc`.
  - Job `sbom-and-notices`: CycloneDX SBOMs — Rust via `cargo-cyclonedx` (or `cargo-about`), npm via `pnpm` + syft over the installers; third-party notices generated with `cargo-about` (Rust, MIT/Apache) + `license-checker`-style output for npm, bundled into the app as a resource (`bundle.resources`) and shown under Help ▸ View Licences (D4b). Notices generation runs **before** the build (it's packaged), SBOMs after.
  - Job `attest`: `actions/attest-build-provenance@<sha>` for every artefact (subject paths), plus SBOM attestations (`actions/attest-sbom`).
  - Job `publish` (environment `release`, needs all): upload to R2 `releases/{version}/{target}-{arch}/` with `wrangler r2 object put` (or `aws s3 cp --endpoint-url` with R2 S3 credentials), set `Content-Type` and `Cache-Control: public, max-age=31536000, immutable` for versioned files; then update `channels/{channel}.json` via D5a's tooling with `rollout_percent` from input (nightly: 100; beta/stable: starts at the input value, default 1 per ADR 0014 for stable); verify the manifest by calling the Worker for each platform with a synthetic install ID; post a summary (versions, sizes, links) to the job summary.
  - Size budget step: fail if any installer exceeds 40 MB (warn at 36 MB).
- `xtask` helpers if useful: `cargo xtask desktop-version` (sync version from tag into `apps/desktop/package.json`/`tauri.conf.json` at build time without committing).
- `docs/guidelines/desktop-release.md` gains a "Cutting a release" section (tag, watch, verify, promote, roll out, pause).
- Clean-VM verification for Linux (Ubuntu 22.04, Ubuntu 24.04, Fedora current): install the deb/rpm, run the AppImage, verify signatures with the runbook commands; recorded in the PR.

**Out of scope** (and which todo covers it)

- Apple signing, notarisation and stapling: [D6c](D6c-macos-signing-and-notarisation.md) (this PR builds macOS unsigned/ad-hoc for nightly only).
- Windows Authenticode signing: [D6d](D6d-windows-signing.md) (this PR builds Windows unsigned for nightly only).
- Download page: [D7b](D7b-settings-renderer-switch-and-download-page.md). First beta: [D7c](D7c-desktop-beta-release.md).
- Flatpak/Snap/stores (non-goals).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2`; `wrangler` for the R2 steps.
- [ ] Pin every action by commit SHA with a version comment; add them to Dependabot's `github-actions` ecosystem.
- [ ] Write the workflow jobs above; reuse D5b's staging workflow pieces (factor shared steps into a composite action under `.github/actions/desktop-build/`).
- [ ] Configure `bundle.targets`, `bundle.linux.deb.depends` (e.g. `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libcups2` if D4c needs it), `bundle.linux.rpm.depends`, `bundle.linux.appimage.bundleMediaFramework: false`, desktop file template from D3a, `bundle.resources` for notices and PDFium.
- [ ] Linux signing steps and verification; publish the debsig policy XML and keyring instructions in the runbook.
- [ ] SBOM, notices, attestations jobs.
- [ ] R2 publishing + manifest update + post-publish verification against the Worker.
- [ ] Nightly-only guard until D6c and D6d merge (a workflow input `allow_unsigned_platforms` that defaults to false for non-nightly channels).
- [ ] Run a nightly end to end; install the nightly AppImage built from the run on a clean VM and let it update to the next nightly (proves the published pipeline works with D5b's updater).

## Acceptance criteria

- [ ] A nightly run builds all six target/arch combinations (or the documented subset if a runner is unavailable, with the gap recorded), publishes to R2 under ADR 0014's layout and updates `channels/nightly.json` automatically.
- [ ] The Worker returns 200 with valid URLs and signatures for every published platform key (post-publish check green).
- [ ] On clean Ubuntu 22.04/24.04 and Fedora VMs: `debsig-verify` (or the documented equivalent), `rpm -K` and the AppImage signature check pass; the deb/rpm install and the app launches (recorded in the PR).
- [ ] Every artefact has a GitHub attestation (`gh attestation verify <file> --repo <owner>/<repo>` passes) and a CycloneDX SBOM.
- [ ] The app bundles a third-party notices file and Help ▸ View Licences shows it.
- [ ] Installer sizes ≤ 40 MB (budget step green; sizes in the job summary).
- [ ] Non-nightly channels cannot be published while macOS/Windows signing is missing (workflow guard tested with a dry run).
- [ ] A nightly-installed AppImage auto-updates to the next nightly.

## Tests

- Workflow dry runs (`workflow_dispatch` with `dry_run: true` skipping uploads) in the PR.
- Unit tests for any xtask/scripts (version derivation: tag/nightly → semver; manifest update).
- Clean-VM verification checklist (Linux) in the PR.

## Docs to update

- `docs/plans/desktop-app.md` §9 D6: tick "Release workflow with tauri-action matrix…", "Linux AppImage (updater) and deb; rpm if the matrix allows", "Upload to R2 with `.sig` files and GitHub attestations; update channel manifests" and the Linux part of "Verify installs on clean VMs".
- `docs/guidelines/desktop-release.md`: "Cutting a release".
- `CLAUDE.md`: release workflow mention; `apps/desktop/README.md`.

## Risks and open questions

- **arm64 runners** (Windows arm64, Linux arm64) availability/cost on GitHub; cross-compiling Tauri for Windows arm64 from x64 works with MSVC; Linux arm64 needs native or cross WebKitGTK — fall back to native arm64 runners or drop arm64 Linux for beta (record).
- **macOS universal vs per-arch**: universal doubles the sidecar/PDFium size; may break the 40 MB budget — measure.
- **rpm signing support** in Tauri's bundler may require a post-processing step; verify current docs.
- **Nightly noise**: avoid publishing identical nightlies (skip if no desktop-relevant changes).
- **Secret exposure in PR workflows**: the release workflow must never run on `pull_request` from forks; environment protection enforces reviewer approval.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
