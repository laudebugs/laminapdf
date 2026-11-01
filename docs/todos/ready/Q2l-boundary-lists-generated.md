# Q2l — Lists that cross the WASM boundary are generated

| Field | Value |
| --- | --- |
| ID | `Q2l` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2l-boundary-lists-generated` |
| PR title | `refactor(wasm,sdk,render): generate every list shared by Rust and TypeScript` |
| Size | M |
| PR | — |

## Goal

Fill rules, line caps and joins, blend modes, artifact kinds and table scopes are kept by hand in both `lamina-wasm/src/build_dto.rs` and `packages/sdk/src/builder.ts`. Some are matched by index with no check, so reordering one silently mis-tags documents, and they cross in two encodings (strings and `u8`). The render error codes and `HAYRO_VERSION` are also hand-copied into TypeScript. This todo defines each list once in Rust and generates the TypeScript side.

## Context

- The [maintainability guideline](../../guidelines/maintainability.md) ("One home per concept"); `lamina-wasm/src/build_dto.rs:185-257`, `packages/sdk/src/builder.ts:68-135`, `lamina-build/src/tag/mod.rs:80` (`TagKind::ALL`), `lamina-build/tests/tagging.rs:875` (scrapes TypeScript source today), `packages/render/src/index.ts:153-178`, `packages/render-hayro/src/index.ts:31`; `cargo xtask gen codes` (the generator to extend).

## Scope

**In scope**

- `cargo xtask gen` writes `packages/sdk/src/generated/builder-enums.ts` and `packages/render/src/generated/codes.ts`; strings on the wire; `HAYRO_VERSION` exported from lamina-render-wasm or checked against `Cargo.toml`; `--check` in `check docs`.

**Out of scope** (and which todo covers it)

- Error shapes ([Q2j](../done/Q2j-boundary-errors-rust.md), [Q2k](../ongoing/Q2k-boundary-errors-ts.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Add `ALL` constants where a Rust enum lacks one.
- [ ] Extend the generator; generate both files; commit them with the `@generated` header.
- [ ] Switch the index-encoded values to strings on both sides; update `@laminapdf/sdk` and lamina-wasm together.
- [ ] Delete the hand lists and the source-scraping test; add `--check` to `check docs`.

## Acceptance criteria

- [ ] No hand-kept boundary list remains; `gen --check` fails if one drifts.

## Post-merge checks

None.

## Tests

The generated-file freshness check; existing builder and tagging tests.

## Docs to update

typescript.md (generated code), the `gen` commands in CLAUDE.md, plan §9.

## Risks and open questions

Changing indices to strings changes the WASM call signatures: change `@laminapdf/sdk` in the same PR.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
