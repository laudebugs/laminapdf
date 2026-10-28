# 0019 — Supply-chain and licence hygiene

- Status: Accepted
- Date: 2026-10-28

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
  - Unlicensed crates and crates whose licence can't be determined fail; `[[licenses.clarify]]` is used only with a link to the evidence.
- **advisories:** the RustSec database; vulnerabilities and yanked versions fail. Unmaintained advisories warn; each ignore entry has an ID, a reason and a review date.
- **bans:** wildcard version requirements denied; duplicate versions warn. Explicit deny entries for copyleft PDF engines and their bindings (for example `mupdf`, `mupdf-sys`) as a belt-and-braces check.
- **sources:** crates.io only. Git dependencies are denied unless listed with a reason; unknown registries are denied.

### npm: licence check

An xtask subcommand reads `pnpm licenses list --json` (production dependencies of every published package and app) and checks it against **the same allow-list**, kept in one place so Rust and npm can't drift. It runs in the PR fast lane. Dev-only tooling is reported but not blocked unless it is bundled into an artefact.

### Pinned toolchains and lockfiles

- `rust-toolchain.toml` pins the Rust toolchain and targets; `packageManager` in the root `package.json` pins pnpm (via Corepack); `engines` and `.nvmrc` pin Node's major version. `wasm-bindgen-cli` and `wasm-opt` are pinned in xtask ([ADR 0002](0002-binding-strategy.md)).
- `Cargo.lock` and `pnpm-lock.yaml` are committed. CI runs `cargo … --locked` and `pnpm install --frozen-lockfile`.
- GitHub Actions are pinned by commit SHA, with the version in a comment.

### Dependency updates: Renovate

We use **Renovate** (with GitHub's Dependabot security *alerts* left on for visibility). Reasons:

- One config covers every ecosystem we have: Cargo, pnpm including `catalog:` entries, GitHub Actions digests, `rust-toolchain.toml`, and custom managers for pinned tools such as `wasm-bindgen-cli`.
- Grouping keeps coupled upgrades in one PR (the wasm-bindgen crate with its CLI; `@tauri-apps/*` with the `tauri` crates), which separate Dependabot ecosystem entries make harder.
- A release-age delay (`minimumReleaseAge`) before proposing a new version reduces exposure to compromised releases that get yanked within days; scheduled lockfile maintenance refreshes transitive dependencies.

Dependabot would also work and needs no app installation; if Renovate becomes a burden, switching is cheap. Check the exact Renovate options against its docs when the config is written.

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
- Upgrades arrive as small, grouped PRs on a schedule, with a delay before brand-new releases; someone still has to review and merge them.
- Fixtures cost a little paperwork each, which slows down "just drop this PDF in". That's intended: a public repo can't hold documents we may not redistribute.
- SBOMs and notices add release-pipeline work in Rounds B and D.

## Alternatives considered

- **cargo-vet** (Tauri appears to use it). Records human audits of each dependency version; stronger, but costly for a small team. Reconsider once the dependency set stabilises.
- **cargo-audit only.** Covers advisories but not licences, bans or sources; cargo-deny includes the same RustSec check.
- **Dependabot** instead of Renovate. Built in and zero-install, with cooldown support; less flexible grouping and no single config for custom-pinned tools. A reasonable fallback.
- **license-checker-rseidelsohn** for npm. Works, but a second allow-list format; `pnpm licenses list` plus our own check keeps one list.
- **Deny-list instead of allow-list** for licences. Misses new or unusual licences by default.
