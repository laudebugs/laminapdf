# 0019 — Supply-chain and licence hygiene

- Status: Accepted
- Date: 2026-10-28
- Amended 2026-11-14 ([X11](../todos/ongoing/X11-lock-github-actions.md)): GitHub Actions are locked by a workflow lockfile, `.github/workflows/actions.lock`, instead of pinned by commit SHA in the workflow files. GitHub made workflow lockfiles [generally available on 2026-10-28](https://github.blog/changelog/2026-10-28-github-actions-workflow-lockfiles-are-generally-available/).

## Context

[ADR 0001](0001-core-language-and-licence.md) commits lamina to `MIT OR Apache-2.0` and forbids AGPL/GPL dependencies in shipped code. That promise is only as good as its enforcement. In the PDF world the hazard is concrete: MuPDF, iText, Ghostscript and PDFsharp ≥ 6.5 are AGPL; Poppler, Xpdf and pdftk are GPL; fpdf2, OpenPDF, Cairo and PoDoFo are LGPL ([research ch. 05 §8.4](../research/pdf/05-creating-pdfs-from-code.md#84-licensing-traps-to-avoid)). One transitive dependency is enough to change what users may do with lamina.

PDF projects also carry licensed *data*: fonts (standard-14 substitutes, fallback fonts), ICC colour profiles and test corpora that often contain third-party documents. Those are not visible to package-manager licence checks ([repo notes §7](../research/architecture/notes/repo-and-packaging.md)).

Beyond licences, a library that parses hostile input and ships in browsers, Workers and a desktop app with an auto-updater is a supply-chain target: compromised releases, typosquats, unpinned CI actions and unreviewed git dependencies. The architecture research recommends cargo-deny (as Oxc uses), an npm licence check and per-release SBOMs ([report: repository section](../research/architecture/app-and-package-architecture.md#the-repository-is-cargo-pnpm-and-a-task-runner-released-through-oidc-and-gated-by-a-pdf-corpus)). The spike already annotated every dependency with its licence in `Cargo.toml` ([spike results](../../spikes/pdf-basics/rust/README.md#results)).

## Decision

### Rust: cargo-deny

A root `deny.toml`, checked in the PR fast lane (`cargo xtask check` runs `cargo deny check`; CI uses the same command):

- **licenses:** an allow-list. Anything not on it fails.
  - Allowed: `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `0BSD`, `Unicode-3.0`, `BSL-1.0`, `CC0-1.0`. `OFL-1.1` only for font-data crates, as a per-crate exception.
  - Not allowed: `GPL-*`, `AGPL-*`, `LGPL-*` in any form. `MPL-2.0` and other weak copyleft licences need a per-crate exception with a written reason, reviewed like an ADR change.
    - Exceptions granted: the UniFFI crates (`uniffi`, `uniffi_bindgen`, `uniffi_core`, `uniffi_internal_macros`, `uniffi_macros`, `uniffi_meta`, `uniffi_pipeline`, `uniffi_udl`) under `MPL-2.0`, owner decision 2026-10-30, for the Swift and Kotlin bindings ([ADR 0029](0029-mobile-bindings-and-packaging.md)); reason in `deny.toml`.
    - Exceptions granted: the crates Tauri 2 itself depends on (`selectors`, `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`) under `MPL-2.0`, owner decision 2026-11-09, for the desktop shell (ADR 0008); found by the [`desktop-ipc` spike](../../spikes/desktop-ipc/README.md#licences); reason in `deny.toml`. The desktop app's third-party notices will carry the MPL-2.0 text (todo D2a).
  - Unlicensed crates and crates whose licence can't be determined fail; `[[licenses.clarify]]` is used only with a link to the evidence.
- **advisories:** the RustSec database; vulnerabilities and yanked versions fail. Unmaintained advisories warn; each ignore entry has an ID, a reason and a review date.
- **bans:** `wildcards = "deny"` with `allow-wildcard-paths = true`, so intra-workspace path dependencies without a version are accepted only in crates that are not published. Internal crates (apps' `src-tauri`, `tools/xtask`, test helpers) set `publish = false`; published crates give every path dependency a `version` (through `[workspace.dependencies]`). Duplicate versions warn. Explicit deny entries for copyleft PDF engines and their bindings (for example `mupdf`, `mupdf-sys`) as a belt-and-braces check.
- **sources:** crates.io only. Git dependencies are denied unless listed with a reason; unknown registries are denied.

### npm: licence check

An xtask subcommand reads `pnpm licenses list --json` (production dependencies of every published package and app) and checks it against **the same allow-list**, kept in one place so Rust and npm can't drift. It runs in the PR fast lane. Dev-only tooling is reported but not blocked unless it is bundled into an artefact.

### Pinned toolchains and lockfiles

- `rust-toolchain.toml` pins the Rust toolchain and targets; `packageManager` in the root `package.json` pins pnpm (via Corepack); Node's version is pinned in `.nvmrc` (used by CI's `setup-node` and by version managers) and declared in `engines`, with `engine-strict=true` in `.npmrc` so pnpm refuses a mismatched Node. `wasm-bindgen-cli` and `wasm-opt` are pinned in xtask ([ADR 0002](0002-binding-strategy.md)).
- `Cargo.lock` and `pnpm-lock.yaml` are committed. CI runs `cargo … --locked` and `pnpm install --frozen-lockfile`.
- GitHub Actions are locked by `.github/workflows/actions.lock`, GitHub's [workflow dependency lockfile](https://docs.github.com/en/actions/concepts/security/workflow-dependency-lockfiles) (amended 2026-11-14, [X11](../todos/ongoing/X11-lock-github-actions.md)). The workflow files name release tags (`actions/checkout@v7.0.1`); the lockfile records the commit each tag resolves to, the repository that owns it, and the actions those actions use in turn, which a SHA in a workflow file never covered. GitHub refuses to run an action that isn't in the lockfile, whose locked commit its tag no longer reaches, or whose repository changed owner. This repository's own composite actions are written `$/.github/actions/<name>`, so the lockfile follows them too; `./` references aren't followed. `gh actions-lock` (the `github/gh-actions-lock` extension, pinned in `tools/xtask/src/doctor.rs`) rewrites the lockfile after a workflow is added or a `uses:` line changes, and the fast lane's `lockfile` job runs `gh actions-lock --verify` on every pull request. Before the amendment, actions were pinned by commit SHA with the version in a comment.

### Dependency updates: Dependabot

We use **Dependabot** (decided by the project owner), configured in `.github/dependabot.yml`, with security alerts and security updates on. Reasons:

- Built into GitHub: nothing to install or grant, no third-party app with write access to the repository.
- It covers our ecosystems: `cargo`, `npm` (pnpm lockfiles), `github-actions`, with one entry per directory. For `github-actions` it updates a workflow's `uses:` tag and its lockfile entry in one PR, but doesn't onboard a workflow that has no lockfile entries yet: `gh actions-lock` does that.
- `groups` keep coupled upgrades in one PR (the wasm-bindgen crate; `@tauri-apps/*`; dev tooling), and `cooldown` delays brand-new releases for a few days, which reduces exposure to compromised releases that get yanked quickly.

What Dependabot does not track, xtask does: the pinned `wasm-bindgen-cli` must match the crate version in `Cargo.lock` (checked by `cargo xtask wasm`, [ADR 0002](0002-binding-strategy.md)), and binaryen, cargo-deny and the toolchain in `rust-toolchain.toml` are bumped by hand in a reviewed PR. Check the exact Dependabot options (pnpm catalog support, grouping syntax) against GitHub's docs when the config is written.

### SBOM and notices (at first release)

Not needed in Round A; required before the first public release (Round B for packages, Round D for desktop):

- CycloneDX SBOMs per release artefact (Rust via `cargo-cyclonedx` or `cargo-about`; npm and installers via syft), attached to the GitHub release alongside the artefact attestations from ADR 0017.
- A generated third-party notices file shipped inside the desktop app and the npm packages that bundle WASM.

### Fonts, ICC profiles and test corpora

- Every file in `fixtures/` has an entry in `fixtures/README.md`: origin, licence, and what it exercises. A file without a known, redistributable licence is not committed ([code organization §6](../guidelines/code-organization.md#6-tests-and-fixtures)).
- Bundled fonts are OFL-1.1 or Apache-2.0 only (for example Noto, Liberation); bundled ICC profiles are freely redistributable ones. Each is listed with its licence in the notices file.
- Large external corpora (pdf.js, SafeDocs, veraPDF suites) stay out of git, fetched by an xtask and pinned by hash, with their licence terms recorded in `fixtures/README.md`.
- Copyleft tools (veraPDF, Poppler, Ghostscript) run only as external processes in CI, never linked or bundled.

## Consequences

- A copyleft or unknown-licence dependency fails CI before review, rather than being found after a release.
- Adding a dependency with an unusual licence is possible but visible: it needs a reviewed exception with a reason.
- Upgrades arrive as small, grouped Dependabot PRs on a schedule, with a cooldown before brand-new releases; someone still has to review and merge them.
- Fixtures cost a little paperwork each, which slows down "just drop this PDF in". That's intended: a public repo can't hold documents we may not redistribute.
- SBOMs and notices add release-pipeline work in Rounds B and D.

## Alternatives considered

- **cargo-vet** (Tauri appears to use it). Records human audits of each dependency version; stronger, but costly for a small team. Reconsider once the dependency set stabilises.
- **cargo-audit only.** Covers advisories but not licences, bans or sources; cargo-deny includes the same RustSec check.
- **Renovate.** One config for every ecosystem including pnpm `catalog:` entries, `rust-toolchain.toml` and custom-pinned tools (regex managers), richer grouping, `minimumReleaseAge` and lockfile maintenance; Tauri uses it. Rejected for now because it needs a GitHub App installed with write access, and Dependabot covers what we need. The documented alternative if the hand-bumped tools above become a burden.
- **license-checker-rseidelsohn** for npm. Works, but a second allow-list format; `pnpm licenses list` plus our own check keeps one list.
- **Deny-list instead of allow-list** for licences. Misses new or unusual licences by default.
