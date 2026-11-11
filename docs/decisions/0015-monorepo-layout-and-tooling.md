# 0015 — Monorepo layout and tooling

- Status: Accepted
- Date: 2026-10-28

## Context

lamina is one repository holding a Rust core (native and WASM), TypeScript packages, a Cloudflare web app, a Tauri desktop app, shared test fixtures and research spikes ([ADR 0001](0001-core-language-and-licence.md), [ADR 0002](0002-binding-strategy.md), [architecture §2](../plans/architecture.md#2-the-architecture-in-one-picture)). Two package managers are unavoidable (Cargo and pnpm). The question is how to lay them out and what, if anything, sits on top.

The architecture research looked at comparable Rust + JS projects. Oxc, Biome, Rolldown and Tauri all use a root Cargo workspace under `crates/`, a pnpm workspace for `packages/` and `apps/`, and a `justfile` or `xtask` as the task runner. None of them puts Turborepo, Nx or moon at the root. Turborepo's Cargo support is behind an experimental flag, with caching off by default because Cargo configuration can change builds without being hashed ([report: repository section](../research/architecture/app-and-package-architecture.md#the-repository-is-cargo-pnpm-and-a-task-runner-released-through-oidc-and-gated-by-a-pdf-corpus); [repo notes §1–2](../research/architecture/notes/repo-and-packaging.md)).

The [code-organization guide](../guidelines/code-organization.md) has since turned that layout into rules and chose a single task entry point, `cargo xtask`, over `just`. The spike showed why shared fixtures belong at the root: the tests that caught the most bugs ran *both* implementations against the *same* bytes ([research ch. 07 §4](../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned)). This ADR records the layout as a decision; the guide holds the detailed rules.

## Decision

1. **Root Cargo workspace with a virtual manifest.** `Cargo.toml` has a `[workspace]` table and no `[package]`, and sets `resolver = "3"` (a virtual manifest can't infer it from an edition). Shared `[workspace.package]` (edition 2024, `rust-version`, licence), `[workspace.dependencies]`, `[workspace.lints]` and profiles live there; members inherit them ([`rust.md`](../guidelines/rust.md)).

   ```toml
   [workspace]
   resolver = "3"
   members = ["crates/*", "apps/desktop/src-tauri", "tools/xtask", "tools/std14-gen"]
   exclude = ["spikes", "fuzz"]
   ```

   Each Tauri app's crate is listed by its path. An `"apps/*/src-tauri"` glob works for Cargo, but Dependabot reads a wildcard only at a path's end and failed every Cargo update on it ([bug log](../bugs/dependabot-cargo-workspace-glob/detail.md)).

2. **Root pnpm workspace.** `pnpm-workspace.yaml` lists `packages/*` and `apps/*` only, with shared versions in the pnpm `catalog:`. The root `package.json` is private and holds dev tooling only. One lockfile per ecosystem (`Cargo.lock`, `pnpm-lock.yaml`), both committed.

3. **Top-level folders:**

   | Folder | Holds | Workspace |
   | --- | --- | --- |
   | `crates/` | Rust crates, flat, directory name = crate name (`laminapdf-*`) | Cargo |
   | `packages/` | TypeScript packages, `@laminapdf/<dir>` | pnpm |
   | `apps/` | Deployables (`web`, `desktop` with `src-tauri/`, `updates`); never depended on | pnpm (+ Cargo for `src-tauri`) |
   | `fixtures/` | Shared test inputs and goldens, with provenance and licence per file | neither |
   | `tools/xtask/` | Repo automation as a Rust binary | Cargo |
   | `fuzz/` | cargo-fuzz targets in the OSS-Fuzz layout, its own workspace (needs nightly) | excluded |
   | `spikes/` | Frozen research code, never imported | excluded from both |
   | `docs/` | Documentation | neither |

4. **One task entry point: `cargo xtask …`.** Cross-language and repo-wide tasks are xtask subcommands (`check`, `test`, `wasm`, `fmt`, later `golden`, `fixtures fetch`, `capabilities`), wired through `.cargo/config.toml` (`xtask = "run --package xtask --"`). Package-local tasks are `package.json` scripts with standard names (`build`, `test`, `typecheck`, `lint`, `format`), which xtask may call with `pnpm -r`. CI runs the same xtask commands developers run; there is no CI-only logic ([code organization §10](../guidelines/code-organization.md#10-scripts)).

5. **No Turborepo, Nx or moon initially.** Cargo already schedules and caches the Rust graph. We revisit (with an ADR) if TypeScript-side CI time or the lack of remote caching becomes a measured problem; Turborepo on the pnpm graph only would be the first candidate.

6. **Pinned toolchains.** `rust-toolchain.toml` pins the Rust toolchain, components and targets (`wasm32-unknown-unknown`, `aarch64-apple-ios`, `aarch64-linux-android`); the root `package.json` pins pnpm via `packageManager`; Node is pinned in `.nvmrc` and declared in `engines`, with `engine-strict=true` in `.npmrc` (details in [ADR 0019](0019-supply-chain-and-licence-hygiene.md)).

**Amendment (owner, 2026-10-31):** the npm scope is **`@laminapdf`**, not `@lamina` as first written: `packages/<dir>` is `@laminapdf/<dir>` (`@laminapdf/sdk`, `@laminapdf/wasm`) and apps use it too (`@laminapdf/docs`), matching the `laminapdf.com` domain. Crate names (`lamina-*`) and the product name are unchanged ([P0](../todos/done/P0-npm-scope-laminapdf.md), [architecture §8 Q3](../plans/architecture.md#8-open-questions-for-the-team)).

**Amendment (owner, 2026-11-01):** the crates are named **`laminapdf-*`** and the façade **`laminapdf`**, not `lamina-*` and `lamina`, because `lamina` on crates.io belongs to an unrelated project. The names now match the npm scope and the domain. The crate directories follow (`crates/laminapdf-<name>`), and so do the Rust paths (`laminapdf_cos::…`). The product is still called lamina, and the `@laminapdf/wasm` module files (`lamina_base`, `lamina_text`, `lamina_extract`, `lamina_render_hayro`) keep their names ([P0b](../todos/done/P0b-crate-names-laminapdf.md)).

**Amendment (owner, 2026-11-01, later):** the product is **Lamina PDF**, and the public names that kept the old one follow it before anything is published ([P0c](../todos/done/P0c-lamina-pdf-names.md)):

- `@laminapdf/wasm`'s modules are `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract` and `lamina_pdf_render_hayro`.
- Type names say `LaminaPdf` in every language: `LaminaPdfError`, `isLaminaPdfError` and `LaminaPdfWarning` in TypeScript, `LaminaPdfId` in Rust, and the planned Swift and Kotlin names (`LaminaPdfView`, `LaminaPdfViewer`).
- Schema IDs start with `laminapdf.` (`laminapdf.error/1`, [ADR 0026](0026-error-and-diagnostic-contract.md)).
- Environment variables start with `LAMINA_PDF_`, the crates' table is `[package.metadata.laminapdf]`, and the planned command-line program is `laminapdf`.
- Cloudflare resources are `lamina-pdf-*`: the docs Worker `lamina-pdf-docs`, and the planned `lamina-pdf-web`, `lamina-pdf-web-staging` and R2 buckets (`lamina-pdf-releases`, …).

The site is laminapdf.com and the repository `laudebugs/laminapdf`.

The detailed rules (naming, dependency direction, public API, tests, configuration inheritance, generated code, when to split a crate) are in [`docs/guidelines/code-organization.md`](../guidelines/code-organization.md). Where this ADR and the guide disagree, update one of them in the same PR so they agree again.

## Consequences

- A single `ls crates/` shows every crate, and a new crate never starts a debate about where it goes. Dependency direction is visible from the folders: arrows point from `apps/` into `packages/` and `crates/`, never back.
- Each toolchain runs from the root without `cd`. A TS package that wraps a crate depends on the crate's *build output* (`packages/wasm`), not its sources.
- xtask itself is plain Rust, so it runs on every OS without a shell, but several subcommands shell out to pinned external tools: `cargo-deny` (`check`), `wasm-bindgen-cli` and `wasm-opt` from binaryen (`wasm`), `cargo-nextest` (`test`) and pnpm/Node (TS tasks). Their versions are pinned in one place in xtask, and `cargo xtask doctor` checks that each is installed at the pinned version and prints (or, with `--install`, runs) the command to fix it. CI installs the same versions. Writing tasks in Rust is more verbose than a `justfile`.
- Because `spikes/` is excluded, the spike's `Cargo.toml` keeps building on its own, and product code can't accidentally depend on it. Porting spike code means copying it into `crates/` and bringing it up to product standards ([code organization §12](../guidelines/code-organization.md#12-spikes-vs-product-code)).
- `fuzz/` stays outside the main workspace so nightly-only `cargo-fuzz` dependencies don't touch the stable build or `Cargo.lock`.
- Without a monorepo build tool, TS builds are not remotely cached. At the planned size this is acceptable; the trigger to revisit is recorded above.

## Alternatives considered

- **`just` as the task runner** (Oxc, Biome, Rolldown). Concise for gluing commands, but it is an extra binary to install and its recipes are shell, which behaves differently on Windows. The guide picked xtask; one entry point matters more than which one.
- **Turborepo, Nx or moon at the root.** Useful for large TS graphs and remote caching, but none of the comparable Rust + JS projects uses one, and their Cargo support is experimental or partial. Added complexity before a measured need.
- **Grouped crate folders** (`crates/core/*`, `crates/bindings/*`). Helps at 50+ crates, costs a placement decision for every new crate. Revisit if the flat list gets hard to scan.
- **Separate repositories** for the core, packages and apps. Cross-cutting changes (a new op exposed in core, WASM and UI) would need coordinated releases across repos; the shared fixtures would have to be duplicated or vendored.
- **Rust inside the JS tree** (`packages/sdk/rust/`). Hides the crate graph from Cargo tooling and couples the crate's layout to one consumer.
