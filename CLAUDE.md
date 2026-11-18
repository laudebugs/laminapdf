# CLAUDE.md

Guidance for Claude Code (and humans) working in this repository. This file holds what every session needs; the details live in [`docs/agents/`](docs/agents/README.md), one file per topic. Read the file for the area you work in before you start.

## What Lamina PDF is

Lamina PDF is a platform for anything and everything PDF: creating PDFs from code, parsing and extracting content, rendering, editing, form filling, signing, and transforming documents. It is a Rust core (bytes in, bytes out; `MIT OR Apache-2.0`; no AGPL/GPL dependencies) under a `laminapdf` façade, with WebAssembly bindings (`@laminapdf/wasm`), a TypeScript API (`@laminapdf/sdk`), the `laminapdf` command-line tool and MCP server, a web reader (`apps/web`), a desktop reader (`apps/desktop`, Tauri) and two sites (`apps/docs`, `apps/site`).

Decisions are recorded as ADRs in [`docs/decisions/`](docs/decisions/); [`docs/agents/decisions.md`](docs/agents/decisions.md) sums up every accepted one. The plan for the rest is in [`docs/plans/architecture.md`](docs/plans/architecture.md) and the plans it links. Don't assume decisions that aren't in an ADR.

## Where the details are

| Read | Before you work on |
| --- | --- |
| [`docs/agents/layout.md`](docs/agents/layout.md) | anything: what each crate, package, app and tool holds, file by file |
| [`docs/agents/commands.md`](docs/agents/commands.md) | running or changing a `cargo xtask` task, a differential, a benchmark, the CLI or an app's dev server |
| [`docs/agents/ci-and-release.md`](docs/agents/ci-and-release.md) | workflows, the PR, nightly and release lanes, the CLI, desktop and web releases |
| [`docs/agents/decisions.md`](docs/agents/decisions.md) | a feature an ADR covers (then read the ADR itself) |
| [`docs/agents/api-reading-and-editing.md`](docs/agents/api-reading-and-editing.md) | `laminapdf-cos`, `-model`, `-content`, `-ops`, `-save`: opening, content streams, ops and undo, page ops, saving |
| [`docs/agents/api-text-and-extraction.md`](docs/agents/api-text-and-extraction.md) | `laminapdf-text`, `laminapdf-extract`: fonts, shaping, text extraction, extraction for retrieval, reader mode |
| [`docs/agents/api-building.md`](docs/agents/api-building.md) | `laminapdf-build`: the builder, tagging, PDF/UA and PDF/A, flowing layout, Markdown and JSON documents |
| [`docs/agents/api-facade-and-cli.md`](docs/agents/api-facade-and-cli.md) | the `laminapdf` façade, `laminapdf-cli` and its MCP server |
| [`docs/agents/api-bindings.md`](docs/agents/api-bindings.md) | `laminapdf-wasm`, `@laminapdf/sdk`, its worker client, `@laminapdf/viewer` |
| [`docs/agents/api-rendering.md`](docs/agents/api-rendering.md) | `laminapdf-render`, the PDFium and hayro adapters and their web packages |
| [`docs/agents/spikes.md`](docs/agents/spikes.md) | a research spike in `spikes/` |

Keep them current in the same PR: a new crate or file of note goes in `layout.md`, a new task or flag in `commands.md`, an API change in its `api-*.md`. Keep this file short; detail belongs in `docs/agents/`.

## Repository layout

```
.
├── CLAUDE.md, AGENTS.md   # this file (AGENTS.md links here); docs/agents/ has the rest
├── Cargo.toml             # Rust workspace (virtual manifest); rust-toolchain.toml pins the toolchain and targets
├── package.json           # private pnpm root (dev tooling); pnpm-workspace.yaml lists packages/*, apps/*
├── .changeset/            # Changesets (ADR 0017); CHANGELOG.md is generated
├── crates/                # Rust crates, flat: crates/laminapdf-<name> (core, edge, app and dev layers)
├── packages/              # npm packages @laminapdf/* (wasm, sdk, render*, viewer, ui, tokens, icons, fonts, cli, …)
├── apps/                  # web reader, desktop reader, update service, docs site, website, component gallery
├── fixtures/              # shared test inputs and goldens (provenance in fixtures/README.md)
├── third_party/           # crates.io releases carried with a patch
├── fuzz/                  # cargo-fuzz targets, its own nightly workspace
├── tools/                 # xtask (the single task entry point) and the other tools
├── spikes/                # research spikes, outside both workspaces
├── .agents/skills/        # the repository's agent skills; .claude/skills links here
├── .github/               # CI lanes, release lanes, deploys; actions locked by workflows/actions.lock
└── docs/                  # research, plans, todos, decisions, design, bugs, guidelines, references, agents
```

The whole tree, file by file, is in [`docs/agents/layout.md`](docs/agents/layout.md).

## Working agreements

- **Always work in a git worktree.** Start every piece of work in a new worktree off main (see docs/guidelines/git-workflow.md). The repo is a bare clone; worktrees are folders inside it: `git worktree add -b <type>/<desc> <type>-<desc> main`. Never `git stash` across worktrees; remove the worktree after merge.
- **Work from the todo board.** Every PR implements one todo from [`docs/todos/`](docs/todos/README.md) (`ready/` → `ongoing/` → `done/`), except bot, CI-maintenance, typo, board-only and progress-snapshot refresh PRs, and bug fixes recorded in `docs/bugs/` ([bug triage](docs/guidelines/bug-triage.md)). Pick one from `ready/` whose dependencies are all `done/` (`cargo xtask todos` lists them); claim it by opening its work PR at once; its first commit moves it to `ongoing/` with `[skip ci]` in the message, so CI first runs on the work's first push. Merge with your own `--body`; otherwise the claim's `[skip ci]` lands on `main` and skips `main`'s workflows. The open PR is the claim, so check `gh pr list` before picking. Move it to `done/` in the same PR before merge, updating links and running `cargo xtask todos`. Plans link to their todos. See [git workflow §2](docs/guidelines/git-workflow.md#pick-work-from-the-todo-board).
- **Pull requests only.** `main` was initialized directly; every change since goes through a branch + PR. Branch names: `<type>/<short-description>` (e.g. `docs/pdf-research`).
- **Review every PR with `/code-review`, then merge.** After opening a PR, run `/code-review high <PR#>` (trivial PRs: `low`) without `--comment`: don't post review comments. Fix the findings in new commits whose messages list each finding and its fix. If there were more than 6 findings, run `/code-review low <PR#>` after the fixes and record those the same way. A non-trivial PR that adds or changes prose people read also gets `/writing-review`, whose findings are counted and fixed separately ([§7](docs/guidelines/git-workflow.md#7-review-and-merge-process)). Run `cargo xtask ci` locally, then squash-merge once CI is green. See [review and merge process](docs/guidelines/git-workflow.md#7-review-and-merge-process).
- **Check visual changes in a browser, from the user's side.** Try every change to what a user sees or touches on screen (the reader, the viewer, the sites, the components) in the built app in a real browser, as a person uses it (mouse, keyboard, real files). Judge it by what they can see and do; tests that only call an API don't prove it. Other changes need their own tests, not a browser. See [bug triage §7](docs/guidelines/bug-triage.md#7-prove-the-fix).
- **Never patch code just to make a test pass.** When a test fails, in CI or locally, find out why. Fix the product if it is wrong. If the test checks something that doesn't make sense, adjust it or remove it, and say why in the commit message. See [bug triage §6](docs/guidelines/bug-triage.md#6-fix-the-cause-not-the-symptom).
- **Run a differential you change.** A PR that changes what a live differential compares runs it locally with `--sample` and commits any exception it needs, each with its reason. A new differential lands with its exceptions file and its sampled step in the full lane's `live-differentials` job. See [git workflow §5](docs/guidelines/git-workflow.md#5-pull-requests).
- **Conventional Commits** for commit messages: `type(scope): summary`.
- **Name the session after its PRs** with the `session-name` skill: `Yanny - [D2d:#421, E1c:#440]`.
- **Document as you go.** Plans go in `docs/plans/`, decisions in `docs/decisions/`, research in `docs/research/`. Update the folder's `README.md` index whenever you add a file.
- **Cite sources.** PDF behavior claims should point to ISO 32000-2 (PDF 2.0) or another primary source. Clearly label recommendations and opinions.
- **Spec first.** When implementing PDF features, read the relevant research chapter in `docs/research/` and the spec section before writing code. Real-world PDFs are frequently malformed — parsers must be tolerant, writers must be strict.
- **Use the project's skills.** Load the `/rust-best-practices` skill before writing or reviewing any Rust code, and the `/tauri-v2` skill before any Tauri work (`apps/desktop`, `src-tauri`, `tauri.conf.json`, commands, IPC, capabilities, the updater, bundling). Load `/writing` before drafting prose people read, and run `/writing-review` on it once the PR is open; [git workflow §7](docs/guidelines/git-workflow.md#7-review-and-merge-process) says which prose and how to handle the findings. Tell subagents to load these skills too. Where a skill and the guidelines in `docs/guidelines/` disagree, the guidelines win: note it in the commit message, and if the skill's rule is better, change the guideline in its own PR.
- **Mind licenses.** Many PDF libraries are AGPL or dual-licensed (e.g. iText, MuPDF/PyMuPDF). Don't add a dependency without checking its license.

## Code organization

Conventions that hold whatever the architecture ADRs decide (crate/package names in them are examples):

- [`docs/guidelines/code-organization.md`](docs/guidelines/code-organization.md): layout (`crates/laminapdf-*`, `packages/` as `@laminapdf/*`, `apps/`, `fixtures/`, `tools/xtask`, `spikes/` outside every workspace), dependency direction (pure bytes-in/bytes-out core, platform code only at the edges), public API, tests, when to split.
- [`docs/guidelines/rust.md`](docs/guidelines/rust.md): edition 2024, workspace `[lints]`, no `unwrap`/`expect`/`panic`/indexing in libraries, `unsafe` only in edge crates with `// SAFETY:`, `thiserror` errors.
- [`docs/guidelines/typescript.md`](docs/guidelines/typescript.md): strict tsconfig, ESM-only, no Node-only APIs in shared packages, named exports only, Biome, Vitest.
- [`docs/guidelines/maintainability.md`](docs/guidelines/maintainability.md): one home per concept (search before writing a helper), size budgets that only go down, one public path per item, no test code in shipped code, comments that say why.
- [`docs/guidelines/docs-writing.md`](docs/guidelines/docs-writing.md) and [`docs/guidelines/examples.md`](docs/guidelines/examples.md): reader-facing docs tell a story in plain English, with no internal IDs; examples use only the public API, carry no test scaffolding and read like code we would merge.


Every crate under `crates/` declares its layer in `Cargo.toml` (`[package.metadata.laminapdf] layer = "core" | "edge" | "app" | "dev"`); xtask applies the bytes-in/bytes-out rules to core crates and refuses a dev crate that could be published.

## Commands

Every repo-wide task goes through `cargo xtask` (source: `tools/xtask`). It runs from any directory and is exactly what CI runs. Needs rustup (the toolchain in `rust-toolchain.toml` installs itself), Node ≥ 22 (`.nvmrc`: 24) and pnpm (version pinned in `package.json`). The everyday ones:

```sh
pnpm install                 # once per worktree: Biome, TypeScript (needed by the TS checks)
cargo xtask check            # everything a PR must pass: `check docs` + `check rust` + `check ts`
cargo xtask check docs       # fast docs and generated-file checks (no network, no build, no node_modules)
cargo xtask check rust       # fmt --check, maintainability, portability, clippy native + wasm32, tests, rustdoc, cargo-deny
cargo xtask check ts         # the platform boundary, biome ci, typecheck and tests of packages/* and apps/*, the sites' builds
cargo xtask fmt              # format Rust and TS/JSON
cargo xtask test             # cargo nextest (or cargo test) + doc tests, then TS tests
cargo xtask wasm             # build @laminapdf/wasm's modules and test @laminapdf/sdk over them
cargo xtask todos            # check the todo graph and print what can start now
cargo xtask gen codes        # regenerate the error contract from fixtures/codes.json (never edit its outputs by hand)
cargo xtask doctor           # are the pinned tools installed? (--install installs the cargo ones)
cargo xtask ci               # the whole fast lane locally: check + wasm + size
```

A single crate: `cargo test -p laminapdf-cos` (integration tests read the shared `fixtures/`; golden files are never rewritten by a test). Every other task and flag is in [`docs/agents/commands.md`](docs/agents/commands.md).
