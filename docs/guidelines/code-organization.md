# Code organization

How Lamina PDF's code is laid out and how its parts may depend on each other. These conventions are meant to hold whatever the architecture ADRs decide. They assume a Rust core (native and WASM), TypeScript packages, a web app and a desktop app in one repository, but they don't fix which crates or packages exist. Every crate and package name below is an **example**, not a decision.

Language-specific rules live in [`rust.md`](rust.md) and [`typescript.md`](typescript.md).

**How to read this guide.** **MUST** and **MUST NOT** are rules: a PR that breaks one needs an ADR or an explicit, reviewed exception. **SHOULD** is the default: you can deviate if you give a reason in the PR. Anything marked *Recommendation* is advice backed by evidence. It is not a rule yet.

## 1. Top-level layout

```
lamina/
├── Cargo.toml              # virtual manifest: [workspace] only, shared deps, lints, profiles
├── Cargo.lock
├── rust-toolchain.toml     # pinned toolchain + targets (wasm32, iOS, Android)
├── rustfmt.toml  clippy.toml
├── package.json            # private root: dev tooling and scripts only, no runtime code
├── pnpm-workspace.yaml     # packages: apps/*, packages/*; dependency catalog
├── pnpm-lock.yaml
├── biome.json              # repo-wide lint/format config
├── tsconfig.base.json      # shared strict compiler options
├── .editorconfig  .gitattributes  .gitignore
│
├── crates/                 # Rust crates, flat, directory name == crate name
│   ├── laminapdf-syntax/   #   e.g. lexer/parser (pure)
│   ├── laminapdf-core/     #   e.g. document model, read/write (pure)
│   ├── laminapdf-wasm/     #   e.g. wasm-bindgen binding (edge)
│   └── laminapdf-cli/      #   e.g. command-line binary (consumer, like an app)
├── packages/               # TypeScript packages published or shared as @laminapdf/*
│   ├── pdf/                #   e.g. @laminapdf/sdk: idiomatic TS API over laminapdf-wasm
│   └── viewer/             #   e.g. @laminapdf/viewer: UI components
├── apps/                   # deployable things, never depended on
│   ├── web/                #   e.g. web app deployed to Cloudflare
│   ├── desktop/            #   e.g. Tauri app; src-tauri/ is a Cargo workspace member
│   └── updates/            #   e.g. update/download Worker for the desktop app
├── fixtures/               # shared test inputs and golden outputs, used by every language
│   ├── corpus/  golden/  README.md (provenance + license per file)
├── fuzz/                   # cargo-fuzz targets (OSS-Fuzz layout): own workspace, excluded from the root one
├── tools/                  # developer tooling
│   └── xtask/              #   cargo xtask: codegen, checks, release helpers
├── docs/                   # documentation (see docs/README.md)
└── spikes/                 # research spikes: outside every workspace, never imported
```

### Rules

- **MUST** keep one Cargo workspace at the root with a *virtual manifest* (a `[workspace]` table and no `[package]`). Because a virtual manifest can't infer the resolver from an edition, it **MUST** set `resolver = "3"` ([Cargo: workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)).
- **MUST** keep one pnpm workspace at the root (`pnpm-workspace.yaml`) and one lockfile per ecosystem (`Cargo.lock`, `pnpm-lock.yaml`), both committed.
- **MUST** put Rust crates directly under `crates/`, with no nesting. The directory name is the crate name, including the `laminapdf-` prefix.
- **MUST** put TypeScript libraries under `packages/` and deployables under `apps/`. Nothing may depend on an app.
- **MUST NOT** nest one package inside another (for example `packages/a/b/package.json`). To group packages, use a glob such as `packages/group/*` and leave the group folder without a `package.json` ([Turborepo: structuring a repository](https://turborepo.dev/docs/crafting-your-repository/structuring-a-repository)).
- **MUST** exclude `spikes/` from both workspaces, and `fuzz/` from the Cargo workspace (`exclude = ["spikes", "fuzz"]` in `Cargo.toml`, and no glob that matches it in `pnpm-workspace.yaml`). Otherwise Cargo treats a spike's `Cargo.toml` as a stray member of the root workspace and refuses to build it.

### Rationale and alternatives

- **Flat `crates/`.** Cargo's crate namespace is flat, so the folder should be too. A single `ls` shows every crate, and a new crate never starts an argument about where it belongs. Large Rust codebases converge on this: rust-analyzer ([matklad, *Large Rust Workspaces*](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)), Typst (`crates/typst-*`), Biome (`crates/biome_*`), Oxc (`crates/oxc_*`) and Tauri (`crates/tauri-*`). *Alternative:* group by layer (`crates/core/*`, `crates/bindings/*`). That helps once there are 50 or more crates, but it costs a decision every time a crate is added. Revisit only if the flat list gets hard to scan.
- **`apps/` + `packages/`.** This is the convention used by Turborepo and pnpm examples, and by Oxc (`apps/`, `packages/`, `napi/`). The split makes dependency direction visible: arrows only point from `apps/` into `packages/` and `crates/`. *Alternative:* Biome publishes per-platform npm packages under `packages/@biomejs/*`. We'll do the same if we ship platform-specific binaries, by grouping them in one folder.
- **Rust and TS side by side, not one inside the other.** Biome, Oxc and Tauri keep `crates/` and `packages/` as siblings at the root. Each toolchain then runs from the root without `cd`, and a JS package that wraps a crate depends on the crate's *build output*, not on its sources.
- **Tauri's `src-tauri/`** is a normal Rust crate and is listed as a workspace member (`members = ["crates/*", "apps/*/src-tauri", "tools/xtask"]`). Its frontend is a normal pnpm package.
- **`tools/xtask`.** Repo automation (codegen, lint gates, release) is written as a Rust binary run with `cargo xtask …`. This is the [cargo-xtask](https://github.com/matklad/cargo-xtask) pattern, and Biome and rust-analyzer use it. It needs no extra toolchain and runs on every OS. Wire it up with an alias in `.cargo/config.toml`: `xtask = "run --package xtask --"`. *Alternatives:* a root `xtask/` folder (Biome), `tasks/` (Oxc), `tools/` (Typst, Deno), or a `justfile` (Biome, Oxc) as a thin front door. Any of these works. Pick one task entry point and document it in CLAUDE.md.
- **`fixtures/` at the root.** The spike showed that the most valuable tests are the ones *both* implementations run against the *same* bytes (golden files, a shared corpus, differential fuzzing; see [research ch. 07](../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned)). Shared fixtures therefore belong to no single crate.

## 2. Boundaries and dependency direction

```
apps/web ─┐               apps/desktop (Tauri)   crates/laminapdf-cli    ← consumers (apps, binaries): never depended on
          ▼                       └──────────┬──────────┘
   packages/* (@laminapdf/…)                 │ depend on library crates only
          │ uses build output of             ▼
          ▼                    crates/laminapdf (façade), laminapdf-ffi …  ← libraries; bindings (ffi, wasm) are edges
   crates/laminapdf-wasm …                      │
          └──────────────┬───────────────────┘
                         ▼
          crates/laminapdf-core, laminapdf-syntax …                        ← pure core: bytes in, bytes out
```

Apps and binaries (`apps/*`, including the Tauri crate in `apps/desktop/src-tauri`, and bin crates such as `laminapdf-cli`) sit at the top: they depend on library crates (a façade such as `laminapdf`, core crates, or a binding they need) and nothing depends on them. A bin crate is never a dependency; `laminapdf-cli` is a sibling consumer of the same libraries.

### Rules

- **MUST:** dependencies point downwards only. The core never depends on a binding, an app, a UI framework or a runtime. Bindings never depend on apps. There are no cycles. Cargo forbids cycles between crates, but not cycles between TS packages, so check those in CI.
- **MUST:** core crates are **bytes in, bytes out**. They take `&[u8]` (or readers the caller supplies) and return bytes or values. They don't use `std::fs`, `std::net`, `std::process`, `std::thread`, clocks (`SystemTime`, `Instant`) or randomness. The spike proved this works: the same core type-checked for wasm32, iOS and Android with no code changes, and `check-targets.sh` enforced it by grepping ([`spikes/pdf-basics/rust/README.md`](../../spikes/pdf-basics/rust/README.md#design-notes)). Product CI **MUST** keep an equivalent check, as an xtask, against the core crates.
- **MUST:** platform-specific code lives at the edges: `wasm-bindgen`, napi, UniFFI and C-ABI shims, Tauri commands, Cloudflare Worker handlers, file pickers, threads and async runtimes. Core crates **MUST NOT** depend on `wasm-bindgen`, `tauri`, `tokio`, `napi` or similar crates, and **MUST NOT** carry `cfg(target_arch = …)` branches for platform behavior. `cargo xtask portability` rejects edge-only crates (`libc`, `getrandom`, …) anywhere in a core crate's dependency graph, with one listed exception: `libc` reached through RustCrypto's `cpufeatures` (pulled in by `aes` and `sha2` for decryption, B2b), which only reads CPU capabilities on aarch64 Linux, Android and Apple targets so the ciphers can use hardware instructions. Any other exception needs the same kind of entry and reason in `tools/xtask/src/workspace.rs`.
- **MUST:** shared TS packages use only web-standard APIs (`Uint8Array`, `TextEncoder`, `fetch`, `crypto.subtle`). Node-only APIs (`node:fs`, `Buffer`, `process`) are allowed only in apps, in CLI packages, or in an explicitly Node-only package such as `@laminapdf/node`. See [`typescript.md`](typescript.md#portability).
- **SHOULD:** when the core needs something from the environment (time for a signature, fonts, randomness for an encryption salt), it takes it as a parameter or a trait or interface the caller implements. It never reaches out for it.
- **MUST NOT** import from another package or crate by relative path (`../../packages/x/src/…`). Depend on it properly (`workspace:*` in `package.json`, `path = …` via `[workspace.dependencies]` in Cargo) and use its public API ([Turborepo](https://turborepo.dev/docs/crafting-your-repository/structuring-a-repository)).

## 3. Naming

| Thing | Convention | Example |
| --- | --- | --- |
| Rust crate (package name and directory) | `kebab-case`, prefixed `laminapdf-` | `crates/laminapdf-core` → `use laminapdf_core::…` |
| Rust module/file | `snake_case` | `xref_stream.rs` |
| Rust binaries, examples, integration test files | `kebab-case` for bins/examples; `snake_case.rs` for test files | `src/bin/lamina-dump.rs`, `tests/round_trip.rs` |
| Cargo feature | short noun, no `use-`/`with-`/`no-` | `std`, `render`, `pdfium` |
| npm package | `@laminapdf/<name>`, directory is `<name>` | `packages/sdk` → `@laminapdf/sdk` |
| Internal-only npm package | `@laminapdf/<name>` with `"private": true` | `@laminapdf/test-utils` |
| TS source file | `kebab-case.ts` | `content-builder.ts` |
| Fixture file | `kebab-case`, describes the case, not the bug number | `xref-stream-hybrid.pdf` |
| Branch | `<type>/<short-description>` | `feat/xref-repair` (see [git-workflow.md](git-workflow.md)) |

- **MUST NOT** use `-rs`/`-rust` in crate names, or meaningless words in feature names. Use `std`, not `use-std`. Negative features such as `no-std` are "practically never correct" ([Rust API Guidelines C-CASE, C-FEATURE](https://rust-lang.github.io/api-guidelines/naming.html)).
- The npm scope is `@laminapdf` (owner, 2026-10-31; [ADR 0015](../decisions/0015-monorepo-layout-and-tooling.md)). *Recommendation:* reserve the `laminapdf` crate name on crates.io and the `@laminapdf` scope on npm before the first public release ([P1](../todos/done/P1-reserve-names-and-trusted-publishing.md)), and check that both are free before relying on them.

## 4. Public API surface and stability

Everything public will be used by someone, so keep the surface small and deliberate.

**Rust** (details in [`rust.md`](rust.md#api-design))

- **MUST:** items are private or `pub(crate)` by default. `pub` is a decision you can defend in review.
- **MUST:** `lib.rs` is a *facade*. Its `//!` crate docs open with an example, it declares modules (mostly private), and it re-exports the public types (`pub use reader::{open, Document};`). Users should rarely need a deep path.
- **MUST:** `missing_docs` is denied workspace-wide, so every public item has a doc comment.
- **MUST:** public enums that may grow (errors, warnings, options) are `#[non_exhaustive]`. Structs that may gain fields have private fields or are `#[non_exhaustive]` ([C-STRUCT-PRIVATE](https://rust-lang.github.io/api-guidelines/future-proofing.html)).
- **MUST:** Cargo features are additive. Enabling a feature never removes API, and any combination compiles. Optional platform support is spelled `std` (enables), never `no_std` ([Cargo: features](https://doc.rust-lang.org/cargo/reference/features.html)). Hide implicit dependency features with `dep:`.
- **SHOULD** run [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks) in CI for every crate with `publish = true`. Unpublished crates set `publish = false`.

**TypeScript** (details in [`typescript.md`](typescript.md#api-design-types-first))

- **MUST:** every package declares its entry points in `package.json` `"exports"`. Anything not listed can't be imported, which is the point ([TS modules reference](https://www.typescriptlang.org/docs/handbook/modules/reference.html)). Use the `"types"` condition first, then `"import"`/`"default"`.
- **MUST:** named exports only, no `export default`.
- **SHOULD:** a single root `src/index.ts` lists its exports *by name* (`export { open, PdfDocument } from "./document.js"`). Avoid `export *` (it leaks internals by accident) and avoid internal barrel files (`index.ts` in sub-folders). Barrels force bundlers and dev servers to load every re-exported module ([Vite: avoid barrel files](https://vite.dev/guide/performance#avoid-barrel-files)). For large packages, prefer subpath exports (`@laminapdf/sdk/forms`).
- **MUST:** set `"sideEffects": false` unless the package really has import-time side effects.

**Across languages.** If a TS package wraps a Rust crate, the TS API is designed for TS users and is not a transliteration of the Rust API. Error codes, warning codes and option names that both expose **MUST** match, because they are part of both APIs. The spike's `PdfErrorCode` union mirrored the Rust `Error` variants; `@laminapdf/sdk` generates its `ErrorCode` union from `fixtures/codes.json` ([ADR 0026](../decisions/0026-error-and-diagnostic-contract.md)).

## 5. Error handling, by layer

| Layer | Rust | TypeScript |
| --- | --- | --- |
| Core library | One `thiserror` enum per crate (or per major module), `#[non_exhaustive]`, with byte offsets where relevant. No panics: no `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/unchecked indexing (all denied in `[workspace.lints.clippy]`) | One error class per package (`class LaminaPdfError extends Error` in `@laminapdf/sdk`) with a string-literal `code` union, the [ADR 0026](../decisions/0026-error-and-diagnostic-contract.md) fields (`kind`, `hint`, `location`, `data`, `docsUrl`) and an `isLaminaPdfError` guard. Throw only `Error` subclasses |
| Binding | Map core errors 1:1 to the target's idiom (a JS `Error` with a `code`, a Swift `Error`) and never drop the code or any other ADR 0026 field | Re-throw binding errors as the package's own error class |
| App / CLI / xtask | `anyhow` with `.context(…)` is fine | Catch at the boundary, log, and show a user-facing message |

Every surface serialises errors and warnings to the one shape in [ADR 0026](../decisions/0026-error-and-diagnostic-contract.md) §1 (`code`, `kind`, `message`, `hint`, `didYouMean`, `location`, `data`, `docsUrl`), with codes from `fixtures/codes.json`. **MUST NOT:** put document text (strings, names or stream data from the input, caller paths) in `message` or `hint`; it goes only in `data` (ADR 0026 §4).

Tolerant reading **MUST** be a written policy. Recoverable problems become *warnings* the caller can inspect (the spike's `Document::warnings()`), not silent fixes and not errors. See [`rust.md`](rust.md#errors) and [`typescript.md`](typescript.md#errors).

## 6. Tests and fixtures

```
crates/laminapdf-core/
├── src/parser.rs          # unit tests at the bottom: #[cfg(test)] mod tests { … }
├── tests/                 # integration tests: public API only, one file per topic
│   ├── common/mod.rs      #   this crate's helpers; shared ones come from laminapdf-test-fixtures
│   ├── corpus.rs
│   └── round_trip.rs
└── benches/
packages/sdk/
├── src/parser.ts
├── src/parser.test.ts     # unit tests colocated with the source
└── test/                  # integration / cross-module / WASM-interop tests
fixtures/
├── README.md              # every file: origin, licence, what it exercises
├── corpus/  expected.json # facts checked by every implementation
└── golden/                # byte-exact expected outputs
```

- **MUST:** Rust unit tests sit in the same file in `#[cfg(test)] mod tests`. Integration tests in `tests/` use only the public API ([Cargo: package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)). Public-API examples are doc tests.
- **MUST:** TS unit tests are colocated (`foo.test.ts` next to `foo.ts`). Integration tests live in `test/`. Vitest is the runner.
- **MUST:** a test name says what behavior it checks, not which bug it fixes: `open_should_repair_when_xref_offset_is_wrong`, `it("repairs a file whose xref offset is wrong")`. Regression tests link the issue in a comment.
- **MUST:** a Rust test helper that more than one crate needs lives in `crates/laminapdf-test-fixtures`: the `fixtures/` directory and its files, hand-built classic files, a seeded PRNG, the byte-slice read of a `ByteSource` fake and the `codes.json` loader. That crate depends on no Lamina PDF crate, isn't published, and is taken as a path-only dev-dependency, so even a crate's unit tests can use it. A helper that needs a crate's own types stays in that crate's `tests/common/`. No crate includes another crate's file with `#[path]`; `cargo xtask check maintainability` checks both rules.
- **MUST:** shared fixtures live in root `fixtures/` and are read by path relative to the repo root (resolved once in a helper). They are never copied into a package. Each file's provenance and licence are recorded in `fixtures/README.md`. PDF corpora often contain third-party content, so a file without a known licence isn't committed.
- **MUST:** golden files are updated only by an explicit command and the diff is reviewed like code. Byte-exact PDF goldens in `fixtures/golden/` are rewritten only by a regenerate command (for example `UPDATE_GOLDEN=1 cargo test` or a `cargo xtask golden` subcommand). Snapshot tools are for text and `Debug` snapshots only: `cargo insta review` (Rust) and `vitest -u` (TS), never for PDF bytes. A test never rewrites its own golden file silently.
- **SHOULD:** behavior that both implementations or bindings expose is tested against the *same* fixture and expectation file, as in the spike (`corpus/expected.json`).
- *Recommendation:* keep large external corpora (pdf.js, SafeDocs, veraPDF suites) **out of git**. Fetch them with an xtask into a gitignored folder, pinned by commit or hash. pdf.js does the same thing for its linked test PDFs (`test/pdfs/*.link`).

## 7. Documentation in code

- **MUST:** Rust public items use `///`. Crates and modules use `//!`. Fallible functions have an `# Errors` section, panicking ones a `# Panics` section, and `unsafe` ones a `# Safety` section ([API Guidelines C-FAILURE](https://rust-lang.github.io/api-guidelines/documentation.html)). Examples are doc tests.
- **MUST:** exported TS symbols have a [TSDoc](https://tsdoc.org/) comment (`/** … */` with `@param`, `@returns`, `@throws`, `@example`, `@remarks`).
- **SHOULD:** comments explain *why* (spec section, invariant, workaround). Cite ISO 32000 clauses as `§7.5.6`. `TODO` comments link an issue: `// TODO(#42): …`.
- Long-form design writing belongs in `docs/`, not in code comments. A crate's `README.md` gives a short overview and links there.

## 8. Configuration files

- **MUST:** repo-wide tool config lives at the root: `Cargo.toml` (`[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]`, `[profile.*]`), `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `biome.json`, `tsconfig.base.json`, `.editorconfig`.
- **MUST:** member configs *inherit*. Crates use `version.workspace = true`, `edition.workspace = true`, `[lints] workspace = true` and `dep = { workspace = true }`. Packages use `"extends": "../../tsconfig.base.json"`, and Biome nested configs use `"extends": "//"` ([Biome: big projects](https://biomejs.dev/guides/big-projects/)). Shared npm dependency versions go in the pnpm `catalog:` ([pnpm catalogs](https://pnpm.io/catalogs)).
- **MUST NOT** override a shared setting locally without a comment saying why.
- **MUST:** secrets never live in config files. Use `.env.example` for the shape. `.env*` is gitignored.
- Deployment config sits with the app it deploys (`apps/web/wrangler.jsonc`, `apps/desktop/src-tauri/tauri.conf.json`).

## 9. Generated code

- **MUST:** generated files start with a header naming their generator, for example `// @generated by `cargo xtask codegen`. DO NOT EDIT.`, and live in a dedicated place (`src/generated/` or a `*_generated.rs` file).
- **SHOULD:** commit generated sources that are expensive to produce or that need special tools (for example typed dictionaries generated from the Arlington model), and add a CI check that regenerating them produces no diff. Biome and rust-analyzer handle their generated code this way (`xtask/codegen`). Mark the files `linguist-generated=true` in `.gitattributes` so they collapse in PR diffs.
- **SHOULD:** don't commit build artefacts (`target/`, `dist/`, `pkg/`, `.wasm`). Packages that consume WASM build it as a task dependency.

## 10. Scripts

- **SHOULD:** cross-language and repo-wide tasks are xtask subcommands (`cargo xtask check`, `cargo xtask codegen`, `cargo xtask fixtures fetch`). Package-local tasks are `package.json` scripts with standard names: `build`, `test`, `typecheck`, `lint`, `format`.
- **MUST:** a task runs from the repo root and from any directory (resolve paths from the script's own location), and is non-interactive so CI can run it.
- **MUST:** shell scripts start with `set -euo pipefail` and pass `shellcheck`. Anything longer than about 50 lines, or with real logic, becomes an xtask.
- **MUST:** CI runs the same entry points developers run. There is no CI-only logic that can't be reproduced locally.

## 11. When to split a crate or package (and when not to)

Split when at least one of these holds:

1. **Different dependencies.** A heavy or optional dependency (a renderer, a crypto stack) shouldn't be compiled by every user. Use a separate crate, or a feature flag if the code is small.
2. **Different platform.** Edge code (bindings such as `laminapdf-wasm` or `laminapdf-ffi`) is always separate from the pure core. The Tauri crate is not an edge crate but an app: it lives under `apps/desktop/src-tauri`, and its platform code (Tauri commands) stays there, on top of the library crates.
3. **Different stability or audience.** A crate you publish with semver guarantees is split from internal helpers (`publish = false`).
4. **Build time.** A crate is the unit of parallel and incremental compilation. Split a big crate along a clean seam when profiling (`cargo build --timings`) shows it's the bottleneck.
5. **A real, acyclic boundary.** The lower part makes sense on its own and has its own tests (for example syntax → model → operations).

Don't split:

- just to organize files. Modules do that, with privacy and no publishing overhead.
- when the two halves share many internal types or must always change together. That's one crate with a `pub(crate)` boundary.
- pre-emptively. Merging crates later is easy. Undoing a premature public crate after a release isn't. *Rule of three:* wait for a second real consumer.
- into `utils`/`common` grab-bags. Put a helper where it is used, or give it a name that says what it does. `laminapdf-test-fixtures` isn't one: every crate's tests use it today, and its name says what it holds (§6).

## 12. Spikes vs product code

- **MUST:** `spikes/` is outside every workspace, and product code never imports, path-depends on or copies from it at build time.
- **MUST:** a spike says in its README that it is a spike, with its date, question and findings. Its findings go into `docs/research/`, and decisions go into ADRs.
- **SHOULD:** when spike code proves worth keeping, re-implement or *copy* it into `crates/` or `packages/` in a normal PR. The code then has to meet these guidelines in full: product lints, docs and tests. Don't move the spike folder. The spike stays as a frozen record.
- Spikes may lower the bar (no docs, `unwrap` in scripts), but they keep the license and secret rules.

## Do / don't

**Do**

- ✅ Add a crate under `crates/laminapdf-<name>`, inherit workspace package fields, lints and dependencies, and add it to `CLAUDE.md` once it matters.
- ✅ Keep core crates and shared packages free of I/O, and pass bytes and callbacks in.
- ✅ Re-export the public API from `lib.rs` / `index.ts` by name.
- ✅ Put a new fixture in `fixtures/` with its provenance, and test it from every implementation.
- ✅ Prefer a module over a crate, and a subpath export over a new package, until a split criterion holds.

**Don't**

- ❌ `use` an app, binding or runtime crate from a core crate.
- ❌ `import "../../other-package/src/x.js"`, `export default`, or `export *` from internals.
- ❌ Commit build output, unlicensed PDFs, or hand-edited generated files.
- ❌ Add an `@laminapdf/utils` or `lamina-common` crate "for later".
- ❌ Depend on anything under `spikes/`.

## References

- Cargo Book: [Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html), [Features](https://doc.rust-lang.org/cargo/reference/features.html), [Package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html): naming (C-CASE, C-FEATURE), future proofing (C-STRUCT-PRIVATE, C-SEALED), documentation (C-CRATE-DOC, C-FAILURE)
- Aleksey Kladov (matklad), [*Large Rust Workspaces*](https://matklad.github.io/2021/08/22/large-rust-workspaces.html) (2021), and [cargo-xtask](https://github.com/matklad/cargo-xtask)
- [Turborepo: Structuring a repository](https://turborepo.dev/docs/crafting-your-repository/structuring-a-repository)
- pnpm: [Workspaces](https://pnpm.io/workspaces), [Catalogs](https://pnpm.io/catalogs)
- [TypeScript modules reference](https://www.typescriptlang.org/docs/handbook/modules/reference.html) (`exports`, `nodenext`, `bundler`)
- [Vite performance guide: avoid barrel files](https://vite.dev/guide/performance#avoid-barrel-files)
- [Biome: configuring big projects](https://biomejs.dev/guides/big-projects/)
- [TSDoc](https://tsdoc.org/)
- Reference repositories, layouts checked on 2026-10-28: [Biome](https://github.com/biomejs/biome) (`crates/`, `packages/@biomejs/*`, `xtask/`), [Oxc](https://github.com/oxc-project/oxc) (`crates/`, `apps/`, `napi/`, `packages/`, `tasks/`), [Tauri](https://github.com/tauri-apps/tauri) (`crates/tauri-*`, `packages/`, `examples/`), [Typst](https://github.com/typst/typst) (`crates/typst-*`, `tests/`, `tools/`, `docs/`), [Deno](https://github.com/denoland/deno) (`cli/`, `ext/`, `libs/`, `runtime/`, `tests/`, `tools/`), [pdf.js](https://github.com/mozilla/pdf.js) (`src/`, `web/`, `test/`, `external/`)
- Lamina PDF: [`spikes/pdf-basics/`](../../spikes/pdf-basics/SPEC.md), [research ch. 05 §8](../research/pdf/05-creating-pdfs-from-code.md#8-implications-for-lamina), [research ch. 07](../research/pdf/07-spike-rust-and-typescript.md)
