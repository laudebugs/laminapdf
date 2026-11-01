# 0017 — Versioning and release: lockstep versions, Changesets with a Cargo sync, the release lane

- Status: Accepted for the versioning part (§1–§6; todo [B7b](../todos/done/B7b-versioning-and-0-1-0-alpha.md), 2026-10-31). The publishing part (§7) is Proposed and completed by [P1](../todos/ready/P1-reserve-names-and-trusted-publishing.md).
- Date: 2026-10-31
- Amended 2026-10-31: `@laminapdf/render`, `@laminapdf/render-hayro` and `@laminapdf/render-pdfium` (C2c) join the lockstep group at `0.1.0-alpha.0`. `@laminapdf/render` and `@laminapdf/render-hayro` are built from the same Rust tree as `lamina_render_hayro` in `@laminapdf/wasm`, so they are only correct at the same commit; `@laminapdf/render-pdfium`'s version is ours (its adapter and the `@laminapdf/render` contract it implements), not PDFium's, and the pinned `@embedpdf/pdfium` stays an ordinary dependency. `version sync` now reads the group from `.changeset/config.json` (a new member needs no code change), merges every member's changelog into the root one (§4), and also writes `@laminapdf/sdk`'s `VERSION` constant (`packages/sdk/src/version.ts`, the worker handshake's version, L2a).

## Context

lamina ships one library through two ecosystems: the Rust crates (`laminapdf` and the core crates under it) and, when this ADR was written, two npm packages (`@laminapdf/wasm`, the WebAssembly modules built from `laminapdf-wasm`, and `@laminapdf/sdk`, the TypeScript API over them; the render packages joined later, see the amendment). They are one product: `@laminapdf/sdk` is only correct with the `@laminapdf/wasm` built from the same commit, and that module is the façade crate compiled to WebAssembly. The error contract ([ADR 0026](0026-error-and-diagnostic-contract.md)) and the op JSON ([ADR 0006](0006-op-log-and-undo.md)) are shared by every surface.

The [repo and packaging notes §5](../research/architecture/notes/repo-and-packaging.md#5-versioning-and-release-changesets-release-plz-cargo-release-semver-checks-provenance-trusted-publishing) compared the common setups: Changesets for npm and release-plz for crates (two tools, two release PRs, independent versions), or one lockstep version driven by Changesets with a script that writes it into `Cargo.toml`, "simpler when `@laminapdf/wasm` is tightly coupled to the core". The notes left open how Changesets handles Cargo packages: it doesn't; it only reads `package.json` files.

On 2026-10-28 the owner decided that nothing is published yet: crates and npm packages stay local (workspace and path dependencies, `pnpm pack` tarballs, git tags) until the owner decides to publish ([architecture: Publishing (later)](../plans/architecture.md#publishing-later)). So the first version, `0.1.0-alpha.0`, is versioned and tagged, and must be proven to install from outside the repository, without a registry.

[ADR 0018 §2](0018-test-strategy.md#2-ci-lanes) already defined a release lane on tags: the full and nightly suites on the release commit, packaging checks (publint, attw) and clean-install smoke tests in each runtime, and "a nightly failure … blocks the next release".

## Decision

### 1. One lockstep version

Every crate in the root Cargo workspace and the npm packages `@laminapdf/wasm`, `@laminapdf/sdk`, `@laminapdf/render`, `@laminapdf/render-hayro` and `@laminapdf/render-pdfium` share **one version** (the render packages since the 2026-10-31 amendment). A release bumps all of them together, even when a change touched one side only. Internal tools (`xtask`, `std14-gen`) are workspace members and carry the same number, which costs nothing (`publish = false`). Separate workspaces outside the lockstep keep their own versions: `fuzz/` (`laminapdf-fuzz`), `tools/render`, the spikes, and the docs app (`@laminapdf/docs`, ignored by Changesets).

One number means a user never has to work out which `@laminapdf/wasm` goes with which `@laminapdf/sdk`, or which crate version a WASM build came from; a bug report names one version.

### 2. Changesets drives the version; `cargo xtask version sync` copies it into Cargo

- **Changesets** (`@changesets/cli`, pinned exactly in the pnpm catalog; MIT) owns the version. `.changeset/config.json` puts those five packages in one `fixed` group, so a changeset for any of them bumps all of them to the same version, and sets `privatePackages: { version: true, tag: true }`, because the packages are `private: true` until P1 and Changesets would otherwise skip them. Contributors add a changeset (`pnpm changeset`) with each user-visible change ([contributing: changesets](../guidelines/contributing.md#changesets)); `pnpm changeset version` consumes them, bumps every member's `package.json` and writes each package's `CHANGELOG.md`.
- **`cargo xtask version sync`** reads the fixed group from `.changeset/config.json` (exactly one `fixed` group, containing at least `@laminapdf/wasm` and `@laminapdf/sdk`; each member found by name under `packages/`, two folders with one member's name an error) and its version (every member's `package.json` must agree) and writes it into `[workspace.package] version` in the root `Cargo.toml`, the `version` of every internal `[workspace.dependencies]` entry (`path = "crates/…"`, which `cargo publish` will need), the internal crates' entries in every tracked `Cargo.lock` (the root's, `fuzz/`'s and the spikes'; edited in place: no network, no build, and `--locked` builds keep working), the root `CHANGELOG.md` (§4), and the string literal of `export const VERSION = "…"` in `packages/sdk/src/version.ts` (the version `@laminapdf/sdk/worker`'s handshake compares; Changesets doesn't know it; a constant rather than a JSON import of `package.json`, which would change how each export condition bundles). The edits are line based, so the manifest's comments survive, and the result is parsed back and checked.
- **`cargo xtask version sync --check`** runs in `cargo xtask check docs` (so in every PR, in both lanes): it fails, naming each file, when a sync would change anything, and when a member crate sets its own version instead of `version.workspace = true`. A version edited by hand anywhere fails CI.

The release preparation is therefore `pnpm changeset version`, then `cargo xtask version sync`, committed together in one PR.

### 3. `0.x` semantics and pre-releases

- Versions follow [SemVer 2.0](https://semver.org/spec/v2.0.0.html) with Cargo's reading of `0.x`: while the major version is 0, a **breaking change bumps the minor** version (`0.1 → 0.2`), and features and fixes bump the patch (`0.1.0 → 0.1.1`). In Changesets terms: `minor` for a breaking change, `patch` for everything else, and never `major` before the owner decides on 1.0.
- **Pre-releases** use the `alpha` identifier, numbered from 0: `0.1.0-alpha.0`, `0.1.0-alpha.1`, … Changesets' pre mode (`pnpm changeset pre enter alpha`, recorded in `.changeset/pre.json`; consumed changesets move to `.changeset/pre/`) produces them; `pnpm changeset pre exit` ends it before `0.1.0`. Cargo and npm both order `0.1.0-alpha.1` below `0.1.0`; the internal dependencies name the exact version anyway (`version sync`, and `pnpm pack` turns `workspace:*` into it).
- What `0.x` promises beyond SemVer (error and warning codes, op JSON versions) is in the [stability policy](../stability-policy.md), a draft until the first registry release ([P2](../todos/ready/P2-first-registry-prerelease.md)).

### 4. Changelog

Changesets writes each member's `packages/<name>/CHANGELOG.md` from the changesets (the default `@changesets/cli/changelog` generator: no GitHub API, no token). A changeset lists only the packages it names, so a fix to `@laminapdf/render` leaves `@laminapdf/sdk`'s changelog with an empty heading for that version. `cargo xtask version sync` therefore merges every member's changelog into the root [`CHANGELOG.md`](../../CHANGELOG.md), the library's changelog: per version, newest first (SemVer order), the union of the members' entries under Major, Minor and Patch Changes, each entry once, without Changesets' dependency-bump entries ("Updated dependencies", `- @laminapdf/wasm@…`), which say nothing in lockstep; a version without entries says "No changes in this release." Nobody edits a changelog by hand; the changesets are the source.

### 5. Tags

A release is tagged `v<version>` on the release commit on `main`, e.g. `v0.1.0-alpha.0`: one tag for the whole library. `cargo xtask version check-tag <tag>` fails unless the tag names the current version. Changesets' own per-package tags (`@laminapdf/sdk@…`, `changeset tag`) are not used. Tags are created by the owner, or by an agent with the owner's permission, after the version PR merges.

### 6. The release lane

`.github/workflows/release.yml` runs on `v*` tags (and on `workflow_dispatch`, with an optional `ref` input, to try it on a branch or an existing tag):

1. **Guard.** `cargo xtask release-guard` asks GitHub (`gh run list --workflow nightly.yml --branch main --event schedule --status completed`) for the latest completed **scheduled** nightly run on `main` (a dispatched run may use short budgets) and refuses to go on unless it succeeded on the release commit or an ancestor of it (`git merge-base --is-ancestor`); `--run <id>` checks one given run under the same rules (how the guard is tested). Then `version check-tag` for a tag.
2. **The fast lane** (`cargo xtask check`, `wasm` with publint `--strict` and attw `--pack`, `size`), **the full lane's suites** (full differential, fuzz at main's budget, round trip, examples and render goldens, capability matrix, `@laminapdf/sdk` in every runtime, `check` on Linux while that runner is on) and **the nightly lane** itself (`nightly.yml`, called as a reusable workflow with `record: false`, so it pushes no benchmark history, saves no fuzz cache and opens no issue, in its own concurrency group) on the release commit.
3. **`cargo xtask verify-install`**: `pnpm pack` tarballs of `@laminapdf/wasm` and `@laminapdf/sdk` (the render packages are not install-verified yet) and the `laminapdf` crate as a `path =` and a `git =` dependency (pinned to the tag) in clean projects outside the checkout, running the quick start in Node 22, 24 and 26, Deno, Bun, Chromium (a Vite production build), workerd (`wrangler dev --local`) and a Rust binary; every output must be byte-identical to `fixtures/golden/examples/hello.pdf`.
4. **No publish step.** The lane's last job only reports that every check passed.

### 7. Publishing (Proposed; completed by P1)

Deferred by the owner's decision of 2026-10-28. P1 records here: name reservation on crates.io and npm, OIDC trusted publishing on both registries (the first crates.io publish is manual), npm provenance, GitHub artifact attestations for the `.wasm` files, and the publish job of the release workflow. Until then nothing in this repository publishes anything.

## Consequences

- One number for the whole library; the sync is a few hundred lines of tested xtask code instead of a second release tool. A version bump touches every crate even when only TypeScript changed; that's the price of lockstep, and cheap while nothing is published.
- Contributors must add a changeset to user-visible changes; forgetting one only means the change is missing from the changelog (nothing fails, by choice: docs, CI and internal refactors need none).
- Changesets' pre mode leaves `.changeset/pre.json` and `.changeset/pre/` in the tree while the version is an alpha.
- `version sync --check` makes a hand-edited version fail every PR, so the two ecosystems can't drift.
- The release lane is long (the nightly suites alone take hours on the shared Mac runners) and is refused while the nightly lane is red; fixing the nightly comes first.
- Crates and packages remain installable only from a checkout, a tarball or a git dependency until P1 and P2.

## Alternatives considered

- **Changesets for npm and release-plz for the crates** (the notes' first option): two tools, two release PRs and independent versions for products that only work at matching versions. release-plz's strengths (per-crate changelogs, publishing) only matter once crates are published and versioned apart; they can be revisited if a crate ever needs its own cadence.
- **cargo-release or a Cargo-first sync (Cargo writes `package.json`)**: Changesets is the only tool with a contributor workflow (one file per change, written in the PR), and the npm side needs it anyway; one direction of sync is simpler than two.
- **Independent versions with a compatibility table**: more numbers to explain, and every bug report would need three versions.
- **`toml_edit` for the manifest edits**: a new dependency for edits a line-based rewrite does safely (it is parsed back and checked) on a manifest we control.
- **Changesets' per-package tags** (`@laminapdf/sdk@0.1.0-alpha.0`): one product, one tag; `v<version>` also names the crates' version.
- **Publishing the alpha**: ruled out by the owner for now (2026-10-28).
