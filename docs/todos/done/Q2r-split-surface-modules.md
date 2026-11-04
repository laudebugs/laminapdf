# Q2r — Split the large xtask, builder, extraction and WASM modules

| Field | Value |
| --- | --- |
| ID | `Q2r` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](./Q2a-maintainability-checks-rust.md), [Q2h](./Q2h-text-writers-dedup.md), [Q2i](./Q2i-one-lexer-one-png-filter.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2r-split-modules` |
| PR title | `refactor(xtask,text,build,wasm): split live, render_diff, extract/font, surface, png and the WASM builder` |
| Size | M |
| PR | [#220](https://github.com/laudebugs/laminapdf/pull/220) |

## Goal

Two xtask modules are over the 800-line budget in lines of non-test code, and the maintainability baseline lists them: `tools/xtask/src/live.rs` (1,115) and `tools/xtask/src/render_diff.rs` (923). This todo splits them and removes their entries, the last two in `[modules]`: [Q2q](../done/Q2q-split-cos-reader.md) and [Q2w](../done/Q2w-split-content-interpreter.md) have already removed the COS reader and `interp.rs`.

It also splits four files that are long but, once tests are excluded, within the budget: `laminapdf-text/src/extract/font.rs` (1,321 lines in total, 786 of non-test code), `laminapdf-build/src/surface/mod.rs` (1,027), `laminapdf-wasm/src/builder.rs` (1,011) and `laminapdf-build/src/surface/png.rs` (872). The review proposed the boundaries.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `tools/xtask/maintainability-baseline.toml`; the files above; `laminapdf-wasm/src/build_dto.rs`, which [Q2l](../done/Q2l-boundary-lists-generated.md) shrinks.

## Scope

**In scope**

- `live.rs` and `render_diff.rs` split into folders of modules under 800 lines of non-test code each (for example the runner, the report and the history apart), and their baseline entries removed.
- `extract/font/{mod,reader,simple,composite,extent}.rs`; `surface/{mod.rs (the API), writer.rs, pending.rs}`; `png/{chunks,decode,interlace}.rs`; `laminapdf-wasm/src/builder/{document,page,image,text}.rs`.

**Out of scope** (and which todo covers it)

- The core modules ([Q2q](../done/Q2q-split-cos-reader.md), [Q2w](../done/Q2w-split-content-interpreter.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Split one file per commit, as pure moves; run the crate's tests after each.
- [x] Run `cargo xtask wasm` after the builder split.
- [x] Remove the two xtask entries from the baseline (`cargo xtask check maintainability` fails until they are gone).

## Acceptance criteria

- [x] No behaviour change; no file over budget; the baseline lists none of these files.

## Post-merge checks

None.

## Tests

Existing suites, `cargo xtask wasm`.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
