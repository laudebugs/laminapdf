# Repository, Packaging, Build, Release and CI Architecture for Lamina (as of 2026-10-28)

Scope: monorepo layout and tooling for a Rust core (native + WASM), TS packages (npm/JSR), a Cloudflare web app and a Tauri desktop app; versioning/release; CI/testing; licensing hygiene. Research ran on 2026-10-28 with a budget of about 15 tool calls. Many sources are secondary, so check the flagged items before relying on them.

---

## 1. Monorepo tooling for Rust + TS (Cargo + pnpm; Turborepo vs Nx vs moon vs just/mise; comparable projects)

### Takeaway
The leading Rust+JS projects (Oxc, Biome, Rolldown, Tauri) all use the same plain setup: a root Cargo workspace (`crates/`), a pnpm workspace (`packages/` or `npm/`/`napi/`), and a `justfile` (or `xtask`) as the task runner. None of these four repos shows Turborepo, Nx or moon at the root. Turborepo has experimental native Cargo support, and moon v2 (Feb 2026) runs Cargo tasks without replacing Cargo. For Lamina, the evidence favors Cargo + pnpm + just as the default, adding Turborepo only for caching the TS/app side.

### Cited Findings
- **Oxc** root contains `crates/`, `napi/`, `npm/`, `packages/`, `apps/`, `tasks/`, plus `Cargo.toml`, `pnpm-workspace.yaml`, `justfile`, `rust-toolchain.toml`, `Cross.toml`, `deny.toml` (cargo-deny), `cliff.toml` (git-cliff) and `oxc_release.toml` (custom release config) — [oxc-project/oxc](https://github.com/oxc-project/oxc)
- **Biome** root contains `crates/`, `packages/`, `xtask/`, `justfile`, `.changeset/` (Changesets), `fuzz/`, `e2e-tests/`, `pnpm-workspace.yaml` — [biomejs/biome](https://github.com/biomejs/biome)
- **Rolldown** root contains `crates/`, `packages/`, `examples/`, `tasks/`, `scripts/`, `justfile`, `pnpm-workspace.yaml`, `.insta.yaml` (snapshot tests), `knip.jsonc`, `cliff.toml`, `oxc_release.toml`, and test submodules (`rollup/`, `test262/`). It publishes per-platform `@rolldown/binding-*` packages including `wasm32-wasi` via napi-rs, uses a CodSpeed badge (benchmarks), and publishes PR preview packages through pkg.pr.new — [rolldown/rolldown](https://github.com/rolldown/rolldown)
- **Tauri** root contains `crates/`, `packages/`, `bench/`, `examples/`, `.changes/` (change files; Tauri's covector tool — tool name not confirmed on the page), `audits/` and `supply-chain/` (likely cargo-vet), `renovate.json`, `ARCHITECTURE.md`, `Cargo.toml` + `pnpm-lock.yaml` — [tauri-apps/tauri](https://github.com/tauri-apps/tauri)
- **Turborepo** native Cargo support is behind the `experimentalCargoWorkspaces` future flag. It finds the crates as packages and maps tasks to Cargo commands, while Cargo still schedules the actual compilation. Caching is turned off unless outputs and inputs are tracked explicitly, because Cargo config can change builds without being hashed — [Turborepo Rust guide](https://turborepo.dev/docs/guides/tools/rust); a canary release note says the Remote Cache can act as an sccache backend for Cargo tasks — [releases.sh v2.10.5-canary.1](https://releases.sh/release/rel_8APR5d7gxP1PQFYIEU8_q-turborepo-v2-10-5-canary-1-adds-cargo-support-and-embedded-sccache-caching); RFC discussion — [vercel/turborepo#13415](https://github.com/vercel/turborepo/discussions/13415). The older approach wraps a crate in a `package.json` and caches `target/release` — [multi-language guide](https://turborepo.dev/docs/guides/multi-language)
- **moon** v2.0 "Phobos" shipped on Feb 18, 2026, after an alpha on Jan 1 and an RC on Jan 26. moon runs cargo commands and does not replace Cargo. Rust support arrived in v1.5, and v1.37 moved it into a plugin that allows a Cargo workspace at any folder depth. The latest release seen was v2.6.2 — [moonrepo blog archive](https://moonrepo.dev/blog/archive), [moon Rust handbook](https://moonrepo.dev/docs/guides/rust/handbook), [v2.6.2](https://newreleases.io/project/github/moonrepo/moon/release/v2.6.2)

### Inferences
- Cargo already caches incrementally and builds the crate graph. The tools mostly add value on the TS/app graph and for remote caching. Keeping `just` as the single entry point (`just build-wasm`, `just test`, `just release`) and calling `pnpm -r`/`turbo` from it matches what Oxc, Biome and Rolldown do.
- Nx has a community Rust plugin, but this research did not verify it (see Gaps). No comparable project found uses Nx.
- The `xtask` pattern (a Rust binary crate run via `cargo xtask`, as in Biome) suits codegen such as AST or TS-type generation. `just` suits gluing commands together.

### Gaps
- I did not fetch the layouts of Typst, Deno, SWC, Zed or pdf.js, nor whether any of them use Turborepo/Nx. From training knowledge (unverified): Typst and Zed are pure Cargo workspaces, SWC uses Cargo + Yarn/pnpm, and pdf.js uses gulp with a single npm package.
- I found no verified status for the Nx Rust plugin (`@monodon/rust`).
- I did not check mise (tool-version pinning) adoption in these repos. Oxc and Rolldown pin through `rust-toolchain.toml` + `.node-version`.

---

## 2. Directory layout conventions (crates/, packages/, apps/, bindings/, fixtures/, xtask; where the WASM build lives)

### Takeaway
The common convention is `crates/` (Rust), `packages/` (published npm packages), `apps/` (deployables), a binding layer (`napi/` in Oxc, or a binding package in `packages/`), and `tasks/` or `xtask/` for tooling. In these projects, WASM/native builds come out of a dedicated binding crate. The binding crate's build output goes into an npm package directory that other workspace packages consume via `workspace:*`.

### Cited Findings
- Oxc separates the Rust core (`crates/`), the Node-API binding crates (`napi/`), the npm distribution (`npm/`), and apps (`apps/`) — [oxc-project/oxc](https://github.com/oxc-project/oxc)
- Rolldown ships its binding as per-platform npm packages (`@rolldown/binding-darwin-*`, `-linux-*`, `-win32-*`, `-wasm32-wasi`) under `packages/` — [rolldown/rolldown](https://github.com/rolldown/rolldown)
- Biome uses `xtask/`, `fuzz/` and `e2e-tests/` as top-level directories — [biomejs/biome](https://github.com/biomejs/biome)
- OSS-Fuzz recommends a top-level `fuzz/` directory with its own Cargo manifest, though the layout is customizable — [OSS-Fuzz Rust guide](https://google.github.io/oss-fuzz//getting-started/new-project-guide/rust-lang/)

### Inferences
- Proposed Lamina layout, synthesized from the above:
  ```
  crates/laminapdf-core, laminapdf-render, laminapdf-*      # pure Rust, no JS deps
  crates/laminapdf-wasm                                # wasm-bindgen facade (cdylib) -> packages/laminapdf-wasm/dist
  crates/lamina-napi (optional)                     # napi-rs facade
  crates/laminapdf-ffi (optional)                      # UniFFI facade for mobile
  packages/lamina (TS API, depends on laminapdf-wasm via workspace:*)
  packages/laminapdf-wasm (generated glue + .wasm; committed only as build output, gitignored)
  apps/web (Cloudflare), apps/desktop (Tauri; src-tauri depends on crates/* by path)
  fixtures/ or tests/corpus/ (golden PDFs, git-LFS or fetched), fuzz/, xtask/ or tasks/, justfile
  ```
- With this layout, the Tauri desktop app links the Rust crates natively and does not use WASM. Only the web app and the npm package consume the WASM build.

### Gaps
- I did not verify how a pure wasm-bindgen project (rather than napi-rs) lays out its generated package, for example Typst's `typst.ts` (third-party) or Ruffle's web package.

---

## 3. Rust→WASM→TS, Rust→Node, Rust→mobile, TS type generation

### Takeaway
The `rustwasm` org was sunset in 2025. **wasm-bindgen** moved to a new `wasm-bindgen` org with new maintainers and is actively released (0.2.11x line). **wasm-pack** also moved to `github.com/wasm-bindgen/wasm-pack` and still ships releases (v0.15.0, "15 May", most likely 2026). It is no longer the only option, and many projects call `wasm-bindgen-cli` + `wasm-opt` directly. For Node-native and WASI, **napi-rs v3** (July 2025) compiles the same crate to `wasm32-wasip1-threads`. For mobile, **UniFFI** (0.31/0.32 in 2026) is the standard. WASI 0.3 was ratified in June 2026, but jco's support is still in progress, so the Component Model is not yet a safe primary path for a browser library.

### Cited Findings
- The rustwasm org was announced as sunsetting on 2025-07-21 and fully archived in September 2025. wasm-bindgen moves to a new `wasm-bindgen` org with new maintainers. wasm-pack, gloo, twiggy, walrus and weedle were to be archived or transferred to existing maintainers — [Inside Rust blog](https://blog.rust-lang.org/inside-rust/2025/07/21/sunsetting-the-rustwasm-github-org)
- wasm-pack now lives at `github.com/wasm-bindgen/wasm-pack`. Its release list is v0.15.0 (15 May, "Latest"), v0.14.0 (20 Jan), v0.13.1 (29 Oct), v0.13.0 (01 Jul). The page shows no years, and the order suggests 0.14 = Jan 2026 and 0.15 = May 2026 — [wasm-pack releases](https://github.com/wasm-bindgen/wasm-pack/releases). Debian moved 0.15.0 to testing in June 2026 — [Debian tracker](https://tracker.debian.org/pkg/wasm-pack)
- wasm-bindgen 0.2.104 added WeakRef bindings and `Symbol.dispose` support. 0.2.110 fixed a `Closure` memory leak and stopped publishing `wasm-bindgen-backend`. The changelog shows 0.2.111 as the latest seen — aggregated in [ecosyste.ms wasm-bindgen](https://dependabot.ecosyste.ms/packages/cargo/wasm-bindgen) and dependency PRs (secondary; check [wasm-bindgen CHANGELOG](https://github.com/wasm-bindgen/wasm-bindgen))
- **tsify / tsify-next**: tsify-next forked tsify while tsify was "in hibernation". Upstream `tsify` later caught up (0.5.6, 2025-10-14) and is now recommended again. The derive `Tsify` + `#[tsify(into_wasm_abi, from_wasm_abi)]` emits `.d.ts` types and respects serde attributes (`rename_all`, `tag`, `untagged`, `flatten`) — [lib.rs tsify](https://lib.rs/crates/tsify), [tsify-next listing](https://crates.io/crates/tsify-next). serde-wasm-bindgen is at 0.6.x — [docs.rs](https://docs.rs/serde-wasm-bindgen)
- Bitwarden's SDK guidance: use tsify for data exchange and wasm-bindgen classes when JS must call Rust or hold live Rust objects. External types without tsify derives (Uuid, DateTime) are missing from the generated definitions — [Bitwarden contributing docs](https://contributing.bitwarden.com/architecture/sdk/web/interoperability)
- **napi-rs v3** (announced 2025-07-07) makes WebAssembly its headline feature: the same crate compiles to WASM "with little code change" — [Announcing NAPI-RS v3](https://napi.rs/blog/announce-v3). Only `wasm32-wasip1-threads` is supported for browser and Node, with threads via SharedArrayBuffer. That means cross-origin isolation (COOP/COEP) is required in browsers. WASM packages carry a `cpu` field so package managers skip them by default — [napi-rs WebAssembly docs](https://napi.rs/docs/concepts/webassembly). The CLI 3.8.2 docs list `wasm32-wasip1` as the threadless runtime, with a default initial memory of 4,000 pages (250 MiB) — [@napi-rs/cli 3.8.2 wasi.md](https://app.unpkg.com/@napi-rs/cli@3.8.2/files/docs/wasi.md)
- rustc considers `wasm32-wasip1-threads` "in flux" because threads in WASI p2 are still being designed — [rustc platform support](https://doc.rust-lang.org/beta/nightly-rustc/rustc_target/spec/targets/wasm32_wasip1_threads/index.html). A `wasm32-wasip3` target spec exists in nightly — [rustc source](https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_target/spec/targets/wasm32_wasip3.rs.html)
- **WASI 0.3.0** was released on 2026-06-11 and ratified by the WASI Subgroup. Wasmtime 46 (2026-06-22) enabled component-model-async by default. The Bytecode Alliance says jco supports all of WASI 0.3 with a default-enabled release "soon", while wasi.dev lists jco as "0.3 in progress, streams landed, futures following". Guest toolchains (Rust, JS) are still catching up — [wasi.dev P3](https://wasi.dev/releases/wasi-p3), [Bytecode Alliance WASI 0.3](https://bytecodealliance.org/articles/WASI-0.3), [wasi.dev languages](https://wasi.dev/languages). One blog dates 0.3 to Feb 2026, which conflicts with the official June date.
- **UniFFI** supports Kotlin, Swift and Python (with Ruby also listed) — [UniFFI 0.30 guide](https://mozilla.github.io/uniffi-rs/0.30/index.html), [docs.rs uniffi 0.31.2](https://docs.rs/crate/uniffi/0.31.2). A July 2026 Renovate PR moved to 0.32.0 with Kotlin-breaking changes: call sites must pass a direct `java.nio.ByteBuffer` instead of `ByteArray`. The same release added a `mutable_records` option — [SnapXRust PR #13](https://github.com/SnapXL/SnapXRust/pull/13)

### Inferences
- Recommended for Lamina:
  - `crates/laminapdf-wasm` built with `cargo build --target wasm32-unknown-unknown --release`, then `wasm-bindgen --target web` (or `bundler`), then `wasm-opt -Oz`, driven from `just`/xtask. This pins the wasm-bindgen CLI version to the crate version, which is required anyway.
  - wasm-pack is acceptable but optional.
  - Use tsify (upstream) for data structs and `#[wasm_bindgen]` classes for handles (Document, Page). This keeps large PDF buffers as `Uint8Array` and avoids serde round-trips.
  - Use `wasm32-unknown-unknown` + wasm-bindgen for the browser and Workers. napi-rs suits a later Node-native package; its browser WASM requires SharedArrayBuffer/COOP-COEP, a burden for library consumers.
  - Use UniFFI for a future mobile SDK.
  - Revisit the Component Model/jco in 2027.
- A hand-written C ABI (cbindgen) is only worth it for C/C++/Python consumers. UniFFI covers Kotlin/Swift/Python with less effort.
- To generate TS types outside the WASM boundary (for example a JSON schema of document models shared with the web app), `ts-rs` and `specta` are options. Their status was not verified here.

### Gaps
- I did not confirm the years of the wasm-pack 0.14/0.15 releases or who maintains it now.
- I did not verify the current wasm-bindgen output targets. It may have a newer ESM/`experimental-nodejs-module` target or support for the ES module integration proposal (source-phase imports).
- I did not verify whether wasm-opt (binaryen) is bundled in wasm-pack 0.15, nor the reference-types/multivalue defaults in Rust ≥1.82 (training knowledge: `wasm32-unknown-unknown` enabled reference-types and multivalue by default around Rust 1.82, and older wasm-opt needed `--enable-reference-types`).
- I did not check the status of `ts-rs` and `specta` in 2026.

---

## 4. Package design (ESM-only, exports maps, runtime compatibility, lazy WASM, bundlers, JSR)

### Takeaway
I found few primary 2026 sources in the budget. The established practice: ship an ESM-only package with a conditional `exports` map, provide `types`, and load the `.wasm` lazily through `new URL('./lamina_bg.wasm', import.meta.url)`. Let callers pass their own `WebAssembly.Module` or URL, for Cloudflare Workers and CSP-restricted sites. Validate the package with publint and `attw --pack`.

### Cited Findings
- `attw --pack` (@arethetypeswrong/cli) and publint catch problems such as a missing `types` condition and broken `exports` — [publish-package skill guide (secondary)](https://claudskills.com/skills/publish-package-registry/SKILL.md)
- napi-rs WASM packages use the `cpu` field so they install only when requested, a pattern for optional WASM fallbacks — [napi-rs WebAssembly docs](https://napi.rs/docs/concepts/webassembly)

### Inferences
- From training knowledge (not verified in this session):
  - Cloudflare Workers import `.wasm` as a module (`import wasm from './x.wasm'`) rather than fetching it, so the package should expose an `init(moduleOrPath)` entry point plus a `"workerd"` export condition.
  - Vite needs either `vite-plugin-wasm` or the `?url` import pattern.
  - webpack 5 supports `asyncWebAssembly`.
  - esbuild needs a loader (`file`/`binary`).
- A dual package design: `@laminapdf/core` (TS API, pure TS where possible), `@laminapdf/wasm` (generated glue), and an optional `@laminapdf/node` (napi), selected via export conditions (`browser`, `workerd`, `node`, `deno`, `default`).
- JSR can publish the TS layer with native TS sources. Its support for shipping `.wasm` assets needs checking.

### Gaps
- I did not verify JSR's current handling of binary/WASM assets, nor npm/JSR dual-publish practice in 2026.
- I did not verify current Vite (v7?) and Rolldown-Vite WASM handling, or Bun/Deno WASM loading specifics.
- I found no 2026 source on tree-shaking a wasm-bindgen glue file. Training knowledge: the glue is a single module, and the binary itself cannot be tree-shaken, so splitting features into separate WASM modules (for example render vs parse) is the only way to cut size.

---

## 5. Versioning and release (Changesets, release-plz, cargo-release, semver checks, provenance, trusted publishing)

### Takeaway
Both registries support OIDC trusted publishing from GitHub Actions. npm reached GA on 2025-07-31 and generates provenance automatically. crates.io launched in July 2025 and added a "trusted publishing only" mode and GitLab support in January 2026. npm is retiring long-lived publish tokens through 2026-27. The common split is Changesets for npm packages (Biome) and release-plz, git-cliff or custom tooling for crates (Oxc and Rolldown use `oxc_release.toml` + git-cliff; Tauri uses `.changes/`, probably covector). Gate releases on cargo-semver-checks, attw and publint.

### Cited Findings
- **npm trusted publishing** reached GA on 2025-07-31 for public and private, scoped and unscoped packages, from GitHub Actions on GitHub-hosted runners. Provenance attestations are generated automatically, so `--provenance` is unnecessary — [GitHub changelog](https://github.blog/changelog/2025-07-31-npm-trusted-publishing-with-oidc-is-generally-available/). It requires npm CLI ≥ 11.5.1 — [npm docs](https://docs.npmjs.com/trusted-publishers). Secondary sources say provenance is only generated from public repos — not confirmed by official docs.
- npm security changes: classic tokens were revoked (reported 2025-12-09). Secondary sources report that from about August 2026, 2FA-bypass granular tokens stop bypassing 2FA for management actions, and from about January 2027 they lose direct publish rights. Trusted publishing does not yet support self-hosted runners — [GitHub changelog 2025-10-10](https://github.blog/changelog/2025-10-10-strengthening-npm-security-important-changes-to-authentication-and-token-management), [DEV writeup (secondary)](https://dev.to/zhangjintao/from-deprecated-npm-classic-tokens-to-oidc-trusted-publishing-a-cicd-troubleshooting-journey-4h8b). npm reportedly added CircleCI trusted publishing on 2026-04-06 (secondary). The exact 2026–27 dates are unverified.
- **crates.io trusted publishing** (RFC 3691) launched in July 2025. A GitHub Actions workflow exchanges its OIDC token for a short-lived publish token, scoped to repo + workflow file (+ optional environment) — [crates.io dev update 2025-07](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07), [RFC 3691](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html). The January 2026 update added GitLab.com support, a per-crate "trusted publishing only" enforcement mode, and blocked the `pull_request_target` and `workflow_run` triggers — [crates.io dev update 2026-01-21](https://blog.rust-lang.org/2026/01/21/crates-io-development-update). The first publish of a crate must be manual, and each crate needs its own trusted-publisher entry — [Rust Forge](https://forge.rust-lang.org/infra/docs/trusted-publishing.html), [Simon Willison](https://simonwillison.net/2025/Jul/12/cratesio-trusted-publishing)
- **release-plz** automates version bumps, CHANGELOGs (via git-cliff), GitHub releases and crates.io publishing through a release PR — [orhun blog](https://blog.orhun.dev/automated-rust-releases/), [lib.rs release-plz](https://www.lib.rs/crates/release-plz). Some workspaces turn off its per-crate changelog (`changelog_update = false`) and generate one root changelog with git-cliff — [OpenLogi writeup (secondary)](https://instagit.com/AprilNEA/OpenLogi/release-plz-tool-versioning-changelog-process.md)
- **cargo-semver-checks** runs in CI through `obi1kenobi/cargo-semver-checks-action@v2`, typically right before `cargo publish`. It can miss some breakages, for example type-information lints were limited per the 2024 project goal — [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks), [Rust project goal 2025h2](https://rust-lang.github.io/goals/2025h2/cargo-semver-checks.html), [predr.ag](https://predr.ag/blog/toward-fearless-cargo-update.md)
- Biome uses `.changeset/` (Changesets) — [biomejs/biome](https://github.com/biomejs/biome). Oxc and Rolldown use `oxc_release.toml` + `cliff.toml` — [oxc](https://github.com/oxc-project/oxc), [rolldown](https://github.com/rolldown/rolldown)

### Inferences
- For Lamina:
  - Use Changesets for the npm packages and release-plz for the crates. Both open "release PRs" and publish on merge through OIDC with no stored secrets.
  - Alternatively, use one unified version (lockstep) driven by Changesets, with a script that syncs `Cargo.toml` versions. This is simpler when `@laminapdf/wasm` is tightly coupled to `laminapdf-core`.
- Reserve the crate names and npm scope early with a manual first publish, then switch each crate to "trusted publishing only".
- Use GitHub environments (`release`) with required reviewers as the human gate. Note the crates.io trigger restrictions.
- For signing: npm provenance (Sigstore) plus GitHub artifact attestations (`actions/attest-build-provenance`) for desktop binaries and `.wasm`. Tauri updater signing is separate (see §6).

### Gaps
- I did not verify that release-plz supports crates.io trusted publishing natively in 2026 (likely via `rust-lang/crates-io-auth-action`, unconfirmed).
- I did not verify the status of cargo-release, nor how Changesets handles non-npm (Cargo) packages.
- I did not check whether crates.io offers Sigstore attestations. api-extractor usage was not researched.

---

## 6. CI on GitHub Actions (target matrix, caching, Playwright, tauri-action; testing strategy)

### Takeaway
The standard stack:
- dtolnay/rust-toolchain + Swatinem/rust-cache (or sccache) for Rust jobs;
- a matrix over `ubuntu`/`macos`/`windows` plus `wasm32-unknown-unknown`, with iOS/Android build-only checks;
- `tauri-apps/tauri-action` for desktop releases, with updater signing keys in secrets;
- Playwright for web and visual tests.

For PDF correctness, Lamina should copy pdf.js's model: a reference-image ("reftest") suite with a `--masterMode` to refresh snapshots. Add differential checks against other engines, cargo-fuzz targets that can be enrolled in OSS-Fuzz, and the veraPDF Docker image for PDF/A output validation.

### Cited Findings
- The official Tauri v2 guide uses a `strategy.matrix.include` list with `tauri-apps/tauri-action@v0` and `contents: write` to build and create GitHub releases — [Tauri v2 GitHub pipeline docs](https://v2.tauri.app/distribute/pipelines/github). The updater signing key is passed as `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — [skills.cat Tauri guide (secondary)](https://skills.cat/skills/dchuk/claude-code-tauri-skills/building-tauri-with-github-actions). macOS runners are Apple Silicon, so the matrix needs an explicit `x86_64-apple-darwin` entry if Intel is supported. Tutorials also add Swatinem/rust-cache — same sources.
- The pdf.js test runner has `--reftest` (reference image viewer) and `--masterMode` (update reference snapshots), plus unit, font and integration tests — [DeepWiki pdf.js testing (secondary)](https://deepwiki.com/mozilla/pdf.js/6.2-testing-infrastructure). Integration tests are Puppeteer-driven, reference tests are pixel comparisons, and everything runs on GitHub Actions — secondary summary only.
- OSS-Fuzz Rust integration: set `language: rust` in `project.yaml` and build with cargo fuzz. Only libFuzzer + AddressSanitizer are supported. The base builder image ships nightly Rust and cargo-fuzz — [OSS-Fuzz Rust guide](https://google.github.io/oss-fuzz//getting-started/new-project-guide/rust-lang/). Biome has a top-level `fuzz/` — [biomejs/biome](https://github.com/biomejs/biome)
- veraPDF ships an official `verapdf/cli` Docker image, which runs with a mounted volume and the PDF path as an argument. A pinned tag v1.28.2 was seen, with an unclear date — [Docker Hub verapdf/cli](https://hub.docker.com/r/verapdf/cli), [veraPDF CLI docs](https://docs.verapdf.org/cli/)
- Rolldown uses CodSpeed for benchmark tracking and insta for snapshot tests — [rolldown/rolldown](https://github.com/rolldown/rolldown)
- Turborepo Remote Cache can back sccache for Cargo tasks (canary) — [releases.sh](https://releases.sh/release/rel_8APR5d7gxP1PQFYIEU8_q-turborepo-v2-10-5-canary-1-adds-cargo-support-and-embedded-sccache-caching)

### Inferences
- Proposed CI tiers:
  1. **PR fast lane:** `cargo fmt/clippy`, `cargo nextest` on Linux, `wasm32-unknown-unknown` build + size budget, TS typecheck/lint/vitest, publint/attw, cargo-deny.
  2. **PR full lane:** an OS matrix (Win/macOS/Linux) and golden-PDF render tests. Rasterize each page and compare to committed PNGs with a perceptual metric such as pixelmatch/SSIM/dssim, with per-fixture thresholds. Add Playwright tests against `apps/web` with screenshot assertions, plus a cargo-semver-checks dry run.
  3. **Nightly:** differential runs against pdf.js, PDFium, qpdf (`--check`, structure) and pypdf (text extraction) over a large corpus, plus extended fuzzing and benchmarks on CodSpeed or bencher.
  4. **Release:** tauri-action builds and changesets/release-plz publish via OIDC.
- iOS/Android: if UniFFI is adopted, start with `cargo check --target aarch64-apple-ios`/`aarch64-linux-android` (via cargo-ndk). Tauri mobile builds are heavier and can be deferred.
- Store a large PDF corpus outside git (an R2 bucket, since Lamina is on Cloudflare, or git-LFS), pinned by hash.

### Gaps
- I did not verify current versions of Swatinem/rust-cache, dtolnay/rust-toolchain, taiki-e/install-action, sccache-action, cargo-nextest, or the tauri-action major version (v0 vs v1 appears inconsistently in localized docs).
- I found no source on Rust PDF crates already in OSS-Fuzz (for example lopdf, pdf-rs), nor on how PDFium, qpdf and pypdf are typically used in CI differential harnesses.
- I did not verify how pdf.js reftests compute image diffs (exact vs tolerance).

---

## 7. Licensing hygiene (cargo-deny, npm license checks, SBOM)

### Takeaway
Among comparable projects, Oxc keeps a `deny.toml` (cargo-deny) at the root, and Tauri keeps `audits/` and `supply-chain/` directories (likely cargo-vet). Lamina should run cargo-deny for licenses, advisories, bans and sources, and an npm license checker. It should also generate an SBOM per release artifact.

### Cited Findings
- Oxc has a root `deny.toml` — [oxc-project/oxc](https://github.com/oxc-project/oxc)
- Tauri has `audits/` and `supply-chain/` (tool not named on the page), `renovate.json` and `dependabot.yml` — [tauri-apps/tauri](https://github.com/tauri-apps/tauri)

### Inferences
- From training knowledge (unverified this session):
  - Use `EmbarkStudios/cargo-deny-action` in CI.
  - Use `cargo-about` or `cargo-cyclonedx` for Rust SBOM and third-party notices, which Tauri apps need in-app.
  - Check npm licenses with `license-checker-rseidelsohn` or pnpm's built-in `pnpm licenses list`.
  - Generate CycloneDX/SPDX SBOMs with `anchore/sbom-action` (syft), attached to GitHub releases.
- PDF-specific hazard: fonts (for example bundled standard-14 substitutes) and ICC profiles carry their own licenses. Track them in `deny.toml` exceptions and a NOTICE file.

### Gaps
- I did not verify any of the SBOM/license tools' 2026 status, or whether Tauri's `supply-chain/` is cargo-vet.
- I did not research the licensing of PDF test corpora (for example the pdf.js corpus, PDFium test files, the veraPDF corpus). This matters before committing fixtures to a public repo.
