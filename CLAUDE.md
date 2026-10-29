# CLAUDE.md

Guidance for Claude Code (and humans) working in this repository.

## What lamina is

lamina is a platform for anything and everything PDF: creating PDFs from code, parsing and extracting content, rendering, editing, form filling, signing, and transforming documents. The first core crates (`lamina-syntax` and `lamina-cos`, ported from the pdf-basics spike, `lamina-model`, `lamina-ops` and `lamina-save`) exist; there are no packages or apps yet. Decisions are recorded as ADRs in `docs/decisions/`; accepted so far: a Rust core, `MIT OR Apache-2.0`, bytes in / bytes out with no AGPL/GPL dependencies ([0001](docs/decisions/0001-core-language-and-licence.md)); wasm-bindgen + tsify bindings ([0002](docs/decisions/0002-binding-strategy.md)); the Cargo + pnpm + `cargo xtask` monorepo ([0015](docs/decisions/0015-monorepo-layout-and-tooling.md)); cargo-deny and licence hygiene ([0019](docs/decisions/0019-supply-chain-and-licence-hygiene.md)); the reader tolerance policy and `Limits` ([0003](docs/decisions/0003-reader-tolerance-and-limits.md)); the document model ([0005](docs/decisions/0005-document-model.md); generation strategy §6: our own builder over `lamina-cos` reusing `pdf-writer`/`subsetter`/`skrifa`/HarfRust, per the `krilla-fit` spike, accepted when its PR merges); the op log and undo ([0006](docs/decisions/0006-op-log-and-undo.md)); the save planner ([0007](docs/decisions/0007-save-planner.md)); the test strategy ([0018](docs/decisions/0018-test-strategy.md)). The plan for the rest is in `docs/plans/architecture.md`, with the shareable libraries (contracts, CLI/MCP, AI extraction, agent docs, mobile; Rounds L1–L6, ADRs 0026–0031 proposed) in `docs/plans/shareable-libraries.md`; don't assume decisions that aren't in an ADR.

## Repository layout

```
.
├── CLAUDE.md            # this file
├── README.md            # project overview
├── Cargo.toml           # Rust workspace (virtual manifest): shared package fields, deps, lints, profiles
├── rust-toolchain.toml  # pinned Rust toolchain + targets (wasm32, iOS, Android)
├── package.json         # private pnpm root: dev tooling only; pnpm-workspace.yaml lists packages/*, apps/*
├── biome.json  tsconfig.base.json  deny.toml  clippy.toml  rustfmt.toml
├── crates/              # Rust crates, flat: crates/lamina-<name>
│   ├── lamina-syntax/   #   core: object model + serialization, lexer, parser, filters (Flate, PNG predictors), Limits
│   ├── lamina-cos/      #   core: reader and overlay: Document over a ByteSource (xref, object streams, repair, page tree,
│   │                    #     warnings), the overlay (CosMut, journal), rebase
│   ├── lamina-model/    #   core: typed views (pages, outline, Info, name/number trees, page labels) and LaminaIds
│   ├── lamina-ops/      #   core: typed serialisable Op (SetInfo, RotatePage), transactions, savepoints, undo/redo
│   └── lamina-save/     #   core: incremental and full writers over the overlay, WriteSink, /ID, rebase; the spike-era Writer
├── fixtures/            # shared test inputs: golden/ (byte-exact), corpus/ + expected.json + exceptions.toml,
│                        #   differential/ (pypdf expectations), ops/v1/ (op JSON samples), codes.json; README.md has provenance
├── fuzz/                # cargo-fuzz targets (OSS-Fuzz layout), its own nightly workspace outside the root one
├── tools/xtask/         # `cargo xtask …`: the single task entry point
├── tools/corpus-facts/  # pypdf facts for fixtures (Python, pinned in requirements.txt); never run by tests
├── tools/fixtures/      # generators for fixtures written for lamina's tests (no dependencies)
├── .github/             # CI fast lane (workflows/ci.yml), full lane + required `gate` (workflows/full.yml),
│                        #   scheduled fuzzing (workflows/fuzz.yml), Dependabot
├── spikes/              # research spikes (throwaway code that informs decisions), outside both workspaces
│   ├── pdf-basics/      # minimal PDF core in Rust + TypeScript, its own frozen fixtures and spec (ported to crates/)
│   ├── krilla-fit/      # Round B1: can krilla generate into lamina's object graph? (answer + measurements in its README)
│   └── hayro-parity/    # Round C1a: hayro vs PDFium render parity on the pdf.js and PDFBox corpora (baseline in its README)
└── docs/                # all documentation (see docs/README.md)
    ├── research/        # deep dives and research notes, one subfolder per topic
    ├── plans/           # roadmaps and feature design docs (each links to its todos)
    ├── todos/           # one file per PR of work: ready/ → ongoing/ → done/ (README.md is the board)
    ├── decisions/       # Architecture Decision Records (NNNN-title.md)
    ├── guidelines/      # how we work (contributing, git workflow, code organization, Rust/TS)
    └── references/      # external specs, tools, corpora
```

## Working agreements

- **Always work in a git worktree.** Start every piece of work in a new worktree off main (see docs/guidelines/git-workflow.md). The repo is a bare clone; worktrees are folders inside it: `git worktree add -b <type>/<desc> <type>-<desc> main`. Never `git stash` across worktrees; remove the worktree after merge.
- **Work from the todo board.** Every PR implements one todo from [`docs/todos/`](docs/todos/README.md) (`ready/` → `ongoing/` → `done/`), except bot, CI-maintenance, typo and board-only PRs. Pick one from `ready/` whose dependencies are all `done/`; claim it with a small claim PR that moves it to `ongoing/` on `main`; move it to `done/` in the work PR before merge, updating links and regenerating the board (`cargo xtask todos`). Plans link to their todos. See [git workflow §2](docs/guidelines/git-workflow.md#pick-work-from-the-todo-board).
- **Pull requests only.** `main` was initialized directly; every change since goes through a branch + PR. Branch names: `<type>/<short-description>` (e.g. `docs/pdf-research`).
- **Review every PR with `/code-review`, then merge.** After opening a PR, run `/code-review high <PR#>` (trivial and claim PRs: `low`) without `--comment`: don't post review comments. Fix the findings in new commits whose messages list each finding and its fix. If there were more than 6 findings, run `/code-review low <PR#>` after the fixes and record those the same way. Run `cargo xtask ci` locally, then squash-merge once CI is green. See [review and merge process](docs/guidelines/git-workflow.md#7-review-and-merge-process).
- **Conventional Commits** for commit messages: `type(scope): summary`.
- **Document as you go.** Plans go in `docs/plans/`, decisions in `docs/decisions/`, research in `docs/research/`. Update the folder's `README.md` index whenever you add a file.
- **Cite sources.** PDF behavior claims should point to ISO 32000-2 (PDF 2.0) or another primary source. Clearly label recommendations and opinions.
- **Spec first.** When implementing PDF features, read the relevant research chapter in `docs/research/` and the spec section before writing code. Real-world PDFs are frequently malformed — parsers must be tolerant, writers must be strict.
- **Use the project's skills.** Load the `/rust-best-practices` skill before writing or reviewing any Rust code, and the `/tauri-v2` skill before any Tauri work (`apps/desktop`, `src-tauri`, `tauri.conf.json`, commands, IPC, capabilities, the updater, bundling). Tell subagents to load them too. Where a skill and the guidelines in `docs/guidelines/` disagree, the guidelines win; record the difference in the guideline.
- **Mind licenses.** Many PDF libraries are AGPL or dual-licensed (e.g. iText, MuPDF/PyMuPDF). Don't add a dependency without checking its license.

## Code organization

Conventions that hold whatever the architecture ADRs decide (crate/package names in them are examples):

- [`docs/guidelines/code-organization.md`](docs/guidelines/code-organization.md): layout (`crates/lamina-*`, `packages/` as `@lamina/*`, `apps/`, `fixtures/`, `tools/xtask`, `spikes/` outside every workspace), dependency direction (pure bytes-in/bytes-out core, platform code only at the edges), public API, tests, when to split.
- [`docs/guidelines/rust.md`](docs/guidelines/rust.md): edition 2024, workspace `[lints]`, no `unwrap`/`expect`/`panic`/indexing in libraries, `unsafe` only in edge crates with `// SAFETY:`, `thiserror` errors.
- [`docs/guidelines/typescript.md`](docs/guidelines/typescript.md): strict tsconfig, ESM-only, no Node-only APIs in shared packages, named exports only, Biome, Vitest.

## Commands

Every repo-wide task goes through `cargo xtask` (source: `tools/xtask`). It runs from any directory and is exactly what CI runs. Needs rustup (the toolchain in `rust-toolchain.toml` installs itself), Node ≥ 22 (`.nvmrc`: 24) and pnpm (version pinned in `package.json`).

```sh
pnpm install                 # once per worktree: Biome, TypeScript (needed by the TS checks)
cargo xtask check            # everything a PR must pass: `check docs` + `check rust` + `check ts`
cargo xtask check docs       # docs-links + todos --check + portable paths (fast; no network)
cargo xtask check rust       # fmt --check, portability, clippy native + wasm32 (-D warnings), tests, rustdoc, cargo-deny
cargo xtask check ts         # biome ci, typecheck and tests of packages/* and apps/*, npm licences
cargo xtask fmt              # format Rust and TS/JSON (fmt --check only verifies)
cargo xtask lint             # clippy (native all-targets + wasm32 core crates) and biome ci
cargo xtask test             # cargo nextest (or cargo test) + doc tests, then TS tests
cargo xtask portability      # core crates: no fs/net/process/thread/clocks, no edge deps, forbid(unsafe),
                             #   cargo check for wasm32-unknown-unknown, aarch64-apple-ios, aarch64-linux-android
cargo xtask wasm             # build the wasm32 crates with the size-optimised `wasm-release` profile
cargo xtask deny             # cargo-deny (licences, advisories, bans, sources) + npm licence check
cargo xtask npm-licenses     # npm licence check only
cargo xtask docs-links       # every relative link and #anchor in tracked *.md resolves (GitHub slugs); --json for tooling
cargo xtask todos            # regenerate docs/todos/README.md and each todo's Blocks from its Depends on links
cargo xtask todos --check    # fail if the board or a Blocks field is stale, or the todo graph is inconsistent
cargo xtask golden --update  # regenerate every golden in fixtures/golden/ from its source, print the list
cargo xtask corpus facts     # rewrite fixtures/corpus/expected.json with pypdf ($LAMINA_PYTHON or python3)
cargo xtask differential     # lamina vs pypdf's precomputed facts (corpus + fixed-seed mutants; part of `test`)
cargo xtask differential --update  # regenerate fixtures/differential/expected.json with pypdf
cargo xtask differential --full [--update]  # the full lane's set: 36 mutants per corpus file (full.json; needs cargo-nextest)
cargo xtask fuzz <target> [--time N]  # cargo +nightly fuzz: lex, parse_object, flate_decode, open, page_tree
cargo xtask fuzz --all --time 120     # every fuzz target in turn (the full lane)
cargo xtask fuzz cmin [<target>]      # minimise fuzz/corpus/ (the full lane does this before caching it on main)
cargo xtask changes [--base <rev>]    # paths changed since the merge base with <rev> (default origin/main, else main);
                                      #   code=true unless all are on the docs allowlist
cargo xtask ci               # the whole fast lane locally: check + wasm
cargo xtask doctor           # are rustc, node, pnpm, cargo-deny, cargo-nextest, wasm-opt … at the pinned versions?
cargo xtask doctor --install # install the pinned cargo tools
```

A single crate: `cargo test -p lamina-cos` or `cargo test -p lamina-save` (integration tests read the shared `fixtures/`; golden files are never rewritten by a test). Hangs are caught by nextest's per-test timeout in `.config/nextest.toml` (robustness tests: 30 s), not by wall-clock asserts (ADR 0018); plain `cargo test` runs the same tests without the timeout.

CI has two PR lanes ([ADR 0018 §2](docs/decisions/0018-test-strategy.md#2-ci-lanes) and its implementation note). The fast lane (`.github/workflows/ci.yml`) runs on every PR, but skips its jobs on docs-only PRs (same `cargo xtask changes` classification); the full lane's `changes` job runs `cargo xtask check docs` on every PR, so docs-only PRs are still checked under `gate`. The full lane (`.github/workflows/full.yml`) runs on every PR too: its `changes` job (`cargo xtask changes`) classifies the PR against the docs allowlist (defined once, in `is_docs` in `tools/xtask/src/changes.rs`); for code PRs and every push to `main` it runs the full differential set and 2 minutes per fuzz target. Every CI job runs on the self-hosted macOS ARM64 runners (`[self-hosted, macOS, ARM64]`, no GitHub-hosted minutes), so `cargo xtask check` runs on macOS only; Linux returns in [X5](docs/todos/ongoing/X5-linux-ci-container.md) (a self-hosted runner in a container on the Mac) and Windows in [X6](docs/todos/ready/X6-windows-ci.md) (parked). Its `gate` job is the required check. A new full-lane job uses exactly `if: ${{ !cancelled() && needs.changes.outputs.code == 'true' }}` and joins `gate`'s `needs` (see the comment at the top of `full.yml`). Both lanes use nextest's `ci` profile (no retries).

Fuzzing (`fuzz/`, see its README) needs nightly Rust and cargo-fuzz (pinned in `doctor.rs`); `.github/workflows/fuzz.yml` runs every target for 10 minutes nightly. Python (pypdf, pinned in `tools/corpus-facts/requirements.txt`) is only needed to regenerate facts; tests read the committed JSON.

Reading API (ADR 0003, 0005): `Document::open(source, OpenOptions)` takes an owned or shared `ByteSource` (`Vec<u8>`, `Arc<[u8]>`, `&'static [u8]`, `Arc<dyn ByteSource>`; in-memory ones are parsed in place, others are read in 64 KiB windows bounded by `Limits::max_read_window`, and a source answering `Pending` makes open or any lazy call fail with `Error::DataPending { ranges }` listing the ranges it needs next: windows, stream data or xref-entry probes); `OpenOptions` holds `Limits` (hard limits fail with `Error::LimitExceeded`, soft ones drop with a warning), strict mode (`Error::Strict`) and the password: the standard security handler (R2–R6, ISO 32000 §7.6) decrypts strings and streams on load, tries it as user then owner password (owner first for R5/R6), and fails with `password-required` or `wrong-password`; `Document::encryption()` and `permissions()` describe the result. Warning and error codes are listed in `fixtures/codes.json`, which a test checks against the Rust enums. Typed views live in `lamina-model`: its `Document` wraps the COS one (and derefs to it) with the ID allocator; `doc.pages()`, `doc.outline()`, `doc.info()`, `doc.page_labels()`, `doc.name_tree(key)`; views raise warnings on the document via `lamina_cos::Document::report`. `LaminaId`s are `p12.0` (kind + object) or path IDs such as `p5.0/Kids/3` for direct dictionaries (ADR 0005 §5).

Editing API (ADR 0005 §3, 0006): the base bytes never change; every write goes through `lamina_cos::Document::edit()` → `CosMut` into the overlay (resolution reads it first; an entry equal to the base is dropped; `is_dirty()`), with a journal (`journal_mark`, `rollback_to`, `take_changes`). `lamina_ops::Document` wraps the model document and is the only write path for callers: `doc.transact(Origin::Local, |tx| { tx.apply(op)?; … })`, `tx.savepoint(…)`, or `begin`/`apply`/`commit`/`rollback`; `undo()`/`redo()` return `UndoOutcome::{Applied, Partial, Skipped}`. Ops (`SetInfo`, `RotatePage`) are serde JSON (`{ "op": "rotatePage", "args": { … } }`, envelope `OpLog { v: 1, ops }`), with frozen samples in `fixtures/ops/v1/`.

Saving API (ADR 0007 §5–§7, ADR 0005 §3; `lamina-save`): `IncrementalWriter` / `FullWriter` over a `lamina_cos::Document` stream to a `WriteSink` (`Vec<u8>`, `IoWrite<W>`; append only); the `Save` trait on `lamina_ops::Document` adds `save_incremental`, `save_full` (write-only sink → `SaveState::SavedNotRebased` until `doc.rebase(bytes)`) and `save_full_and_rebase` (a `ReadableSink`), B3b's names until B3c's `Document::save` and planner. A failed save leaves the document `Dirty`; a failed rebase after a complete write returns `SaveError::NotRebased` with the output; undo history survives only an incremental save that freed nothing. `SaveOptions` (`IncrementalOutput`, `Renumber::{Keep, Compact}`, `XrefKind::{Stream, Classic}`, `XrefEol`), `SaveReport`. `/ID[0]` is kept, `/ID[1]` is a content hash; output is byte-deterministic. Encrypted bases are refused by both writers, repaired ones by the incremental one (a save with nothing to append writes the base unchanged). The spike-era `Writer` / `incremental_update` (now in `lamina-save`) build test files from scratch; `lamina-cos`, `lamina-model` and `lamina-ops` reach it through a dev-dependency.

Every crate under `crates/` declares its layer in `Cargo.toml` (`[package.metadata.lamina] layer = "core" | "edge" | "app"`); xtask applies the bytes-in/bytes-out rules to core crates. External tool versions are pinned in `tools/xtask/src/doctor.rs` (CI installs the same ones): `cargo-deny` is required by `check`; `cargo-nextest` is optional (falls back to `cargo test`), except for `differential --full`, which needs it.

Research spikes (`spikes/`) are throwaway-quality APIs used to inform decisions, not the product. They are outside both workspaces; CI still runs the pdf-basics spike's checks in a separate job.

`spikes/krilla-fit/` — krilla vs a builder over `lamina-cos` (Round B1); standalone, not in CI: `scripts/fetch-font.sh` once, then `cargo run --release` prints the measurements (see its README).

`spikes/hayro-parity/` — hayro vs PDFium render parity (Round C1a); standalone, not in CI: `scripts/fetch-pdfium.sh && scripts/fetch-corpora.sh` once (downloads into gitignored folders), then `cargo run --release` writes `out/dpi96/results.csv` and `summary.md` (see its README).

`spikes/pdf-basics/` — the same minimal PDF core in Rust and TypeScript (see its `SPEC.md`):

```sh
# Rust (needs rustup; targets wasm32-unknown-unknown, aarch64-apple-ios, aarch64-linux-android)
cd spikes/pdf-basics/rust
cargo test
cargo clippy --all-targets --all-features --locked -- -D warnings
scripts/check-targets.sh                               # everything: cross-target checks, wasm build, fmt, clippy, tests
cargo build --release --target wasm32-unknown-unknown  # wasm loaded by the TS tests

# TypeScript (Node ≥ 20, pnpm)
cd spikes/pdf-basics/ts
pnpm install
pnpm typecheck && pnpm lint && pnpm test               # WASM interop tests run once the Rust wasm is built
pnpm size                                              # min+gz bundle size
```

Rust code follows the `rust-best-practices` skill (no `unwrap`/`expect` in library code, clippy clean with `-D warnings`). Library code in both languages is bytes-in/bytes-out: no file system, network or Node-only APIs, so it runs on web, mobile and desktop.
