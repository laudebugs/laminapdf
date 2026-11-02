# Q2l — Lists that cross the WASM boundary are generated

| Field | Value |
| --- | --- |
| ID | `Q2l` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2l-boundary-lists-generated` |
| PR title | `refactor(wasm,sdk,render): generate every list shared by Rust and TypeScript` |
| Size | M |
| PR | [#195](https://github.com/laudebugs/laminapdf/pull/195) |

## Goal

Fill rules, line caps and joins, blend modes, artifact kinds and table scopes are kept by hand in both `laminapdf-wasm/src/build_dto.rs` and `packages/sdk/src/builder.ts`. Some are matched by index with no check, so reordering one silently mis-tags documents, and they cross in two encodings (strings and `u8`). The render error codes and `HAYRO_VERSION` are also hand-copied into TypeScript. This todo defines each list once in Rust and generates the TypeScript side.

## Context

- The [maintainability guideline](../../guidelines/maintainability.md) ("One home per concept"); `laminapdf-wasm/src/build_dto.rs:185-257`, `packages/sdk/src/builder.ts:68-135`, `laminapdf-build/src/tag/mod.rs:80` (`TagKind::ALL`), `laminapdf-build/tests/tagging.rs:875` (scrapes TypeScript source today), `packages/render/src/index.ts:153-178`, `packages/render-hayro/src/index.ts:31`; `cargo xtask gen codes` (the generator to extend).

## Scope

**In scope**

- `cargo xtask gen` writes `packages/sdk/src/generated/builder-enums.ts` and `packages/render/src/generated/codes.ts` (the render codes, their kinds and the `docsUrl` base `RenderError.toDiagnostic()` uses, all hand-copied in `packages/render/src/index.ts` today); strings on the wire; `HAYRO_VERSION` exported from laminapdf-render-wasm or checked against `Cargo.toml`; `--check` in `check docs`.

**Out of scope** (and which todo covers it)

- Error shapes ([Q2j](../done/Q2j-boundary-errors-rust.md), [Q2k](../done/Q2k-boundary-errors-ts.md)).

## Tasks

- [x] Claim by opening the work PR (#195) from a fresh worktree off `main`.
- [x] Add `ALL` to `FillRule`, `LineCap`, `LineJoin`, `BlendMode`, `ArtifactKind` and `TableScope` (`TagKind` had one). An exhaustive-match test per enum keeps each `ALL` complete, and a laminapdf-wasm unit test checks that each name table names every variant once.
- [x] Extend the generator. `cargo xtask gen codes` also writes `packages/render/src/generated/codes.ts`, from `fixtures/codes.json` and the codes laminapdf-render's `RenderError::error_code` returns. The new `cargo xtask gen boundary` reads laminapdf-wasm's name tables and `TagKind::as_str` from the source, without a build, and writes `packages/sdk/src/generated/builder-enums.ts` and `packages/render-hayro/src/generated/version.ts` (`HAYRO_VERSION` from the workspace's hayro pin, which laminapdf-render's `HayroBackend::version` must also return). Each file carries the `@generated` header.
- [x] Send structure types, table scopes and artifact kinds as names: `beginTag` and `beginArtifact` take strings, and `@laminapdf/sdk` changes with them. Fill rules, caps, joins and blend modes already travelled as names.
- [x] Delete the hand lists: `@laminapdf/sdk`'s builder lists, `@laminapdf/render`'s code tables and its `test/codes.test.ts`, and laminapdf-build's test that scraped `builder.ts`. The render generator lists only `disposed` itself (the web adapters alone throw it). `check docs` runs `gen boundary --check` beside `gen codes --check`.

The name tables stay in laminapdf-wasm's `src/build_dto.rs`, their one home, so that file doesn't shrink as [Q2r](../ready/Q2r-split-surface-modules.md) expected: it loses the index decoding and gains the two new tables and their test (849 → 866 lines).

## Acceptance criteria

- [x] No hand-kept boundary list remains; `gen --check` fails if one drifts.

## Post-merge checks

None.

## Tests

The generated-file freshness check; existing builder and tagging tests.

## Docs to update

typescript.md (generated code), the `gen` commands in CLAUDE.md, plan §9.

## Risks and open questions

Changing indices to strings changes the WASM call signatures: change `@laminapdf/sdk` in the same PR.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
