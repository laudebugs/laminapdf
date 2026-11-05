# X1 — Docs link checker and todo-board check in `cargo xtask`, run in CI

| Field | Value |
| --- | --- |
| ID | `X1` |
| Status | `done` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting) |
| Round / milestone | Cross-cutting (any time; recommended first) |
| Depends on | None |
| ADRs | Follows [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (xtask is the single entry point; CI runs what xtask runs) |
| Branch | `feat/x1-docs-link-checker` |
| PR title | `feat(xtask): docs link checker and todo-board check (X1)` |
| Size | M (≈ 250–450 lines incl. tests) |
| PR | [#17](https://github.com/laudebugs/lamina/pull/17) |

## Goal

Catch broken documentation links before merge. The todo board makes link rot likely: every todo moves `ready/` → `ongoing/` → `done/`, and plans, the board README and other todos link to it by path. `cargo xtask docs-links` checks every relative link and `#anchor` in every tracked `*.md` file (GitHub's anchor slug rules, including duplicate-heading suffixes), and runs in the CI fast lane.

## Context

- [Todo board](../README.md) and [git workflow §2 "Pick work from the todo board"](../../guidelines/git-workflow.md#pick-work-from-the-todo-board): when files move and which links must be updated.
- [ADR 0015](../../decisions/0015-monorepo-layout-and-tooling.md): xtask conventions; [code organisation §10 scripts](../../guidelines/code-organization.md#10-scripts).
- A throwaway Python checker was used to write the todos; this replaces it with a tested Rust implementation.

## Scope

**In scope**

- `tools/xtask/src/docs_links.rs`: walk tracked Markdown (`git ls-files '*.md'`, excluding `spikes/*/target`, `node_modules`), parse links with `pulldown-cmark` (inline links, reference links, images; skip code spans/blocks and autolinks/absolute URLs), resolve relative paths against the file, check the target exists (file or directory), and for `#anchor` on `.md` targets compute GitHub-style slugs (lowercase; remove punctuation except `-` and `_`; spaces → `-`; Unicode letters kept; inline code/links reduced to text; duplicate headings get `-1`, `-2`, …) and check the anchor exists; same-file `#anchor` links too; explicit HTML anchors (`<a id="…">`) supported.
- Output: `path:line: broken link <target> (missing file | missing anchor)`; non-zero exit on any error; `--json` for tooling.
- `cargo xtask check` runs it (fast lane); documented in `CLAUDE.md`.
- Optional (flag): warn on links to `docs/todos/<state>/X.md` whose file lives in another state folder, suggesting the right path.

**Out of scope**

- External URL checking (network; flaky): [X3c](X3c-nightly-link-check.md), a nightly job.
- The docs site's own checker: [B7a](./B7a-docs-site.md).

## Tasks

- [x] Add `pulldown-cmark` to xtask deps (licence check)
- [x] Implement `docs_links.rs` with a slugger module and unit tests (cases: `## 7. Encryption (§7.6)` → `7-encryption-76`, `Round A — Foundations` → `round-a--foundations`, `B6. WASM and @lamina/pdf` → `b6-wasm-and-laminapdf`, duplicates, emoji, inline code)
- [x] Wire `docs-links` into `main.rs`, `check`, and `ci`
- [x] Fix any existing broken links found on `main` in the same PR (none found: 257 files clean)
- [x] Document in `CLAUDE.md` commands and the todo README
- [x] Add `cargo xtask todos` (same xtask crate, no new runtime deps beyond what's allowed): parse every `docs/todos/{ready,ongoing,done}/*.md` header table, check that **Status** matches the folder, that every **Depends on** ID exists, that the graph has no cycles, that **Blocks** is exactly the reverse of **Depends on**, and that no `done` todo depends on an unfinished one; regenerate `docs/todos/README.md` (the board: "Can start now", ongoing, one table per plan). `cargo xtask todos --check` fails CI when the board or any Blocks field is stale. Port the reference script in the appendix (explicit Depends-on links only, reverse edges, cycle check, board text as committed).

## Acceptance criteria

- [x] `cargo xtask docs-links` exits 0 on `main` after this PR's fixes, and non-zero with a precise message for a broken file link and a broken anchor (integration test on a temp tree)
- [x] Slug unit tests cover every heading style used in `docs/` (agreement with GitHub verified by hand for ten real headings, listed in the PR)
- [x] Runs in the fast lane in under 5 seconds on the repository
- [x] `cargo xtask todos --check` passes on main and fails on: a todo in the wrong folder for its Status, an unknown dependency ID, a dependency cycle, a stale Blocks field, a stale board.

## Post-merge checks

None.

## Tests

Unit tests for the slugger and link extraction; an integration test building a temp directory of Markdown files.

## Docs to update

- [x] [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting): tick the link-checker item
- [x] `CLAUDE.md` (commands), [todo board README](../README.md) (mention the check)

## Risks and open questions

- GitHub's slug algorithm has edge cases (emoji, some punctuation); keep the implementation matching observed behaviour and add cases as they appear.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Appendix: implementation

The one-off reference board script that used to be here is ported to [`tools/xtask/src/todos.rs`](../../../tools/xtask/src/todos.rs) (`cargo xtask todos [--check]`); the link checker is [`tools/xtask/src/docs_links.rs`](../../../tools/xtask/src/docs_links.rs) with the slugger in [`docs_links/slug.rs`](../../../tools/xtask/src/docs_links/slug.rs). Differences from the script: a **Status** may carry a note after the state (`` `ready` — blocked on … ``), the file name must start with the ID, duplicate IDs are an error, only tracked files are read (`git ls-files`, as for the link check), and an empty Ongoing section is followed by one blank line instead of two.
