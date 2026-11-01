# Q2r — Split the large builder, extraction and WASM modules

| Field | Value |
| --- | --- |
| ID | `Q2r` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2h](./Q2h-text-writers-dedup.md), [Q2i](./Q2i-one-lexer-one-png-filter.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2r-split-surface` |
| PR title | `refactor(text,build,wasm): split extract/font, surface, png and the WASM builder` |
| Size | M |
| PR | — |

## Goal

Five more modules are over the 800-line budget: `lamina-text/src/extract/font.rs` (1,321), `lamina-build/src/surface/mod.rs` (1,027) and `png.rs` (872), `lamina-wasm/src/builder.rs` (1,011) and `build_dto.rs` (748, shrinks with [Q2l](./Q2l-boundary-lists-generated.md)). Split them along the boundaries the review proposed.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); the files above.

## Scope

**In scope**

- `extract/font/{mod,reader,simple,composite,extent}.rs`; `surface/{mod (API), writer.rs, pending.rs}`; `png/{chunks,decode,interlace}.rs` (after Q2i's unfilter reuse); `lamina-wasm/src/builder/{document,page,image,text}.rs`; `build_dto` by topic if still large; shrink the baseline.

**Out of scope** (and which todo covers it)

- Core modules ([Q2q](./Q2q-split-large-rust-modules-core.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Pure moves, one module per commit; run all tests and the WASM build.

## Acceptance criteria

- [ ] No behaviour change; every file under budget; the baseline lists none of these.

## Post-merge checks

None.

## Tests

Existing suites, `cargo xtask wasm`.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
