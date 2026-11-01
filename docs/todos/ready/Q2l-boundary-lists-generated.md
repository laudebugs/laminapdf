# Q2l — Lists that cross the WASM boundary are generated or pinned

| Field | Value |
| --- | --- |
| ID | `Q2l` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2l-boundary-lists` |
| PR title | `refactor(wasm,sdk,render): generate or pin every list shared by Rust and TypeScript` |
| Size | M |
| PR | — |

## Goal

Fill rules, line caps and joins, blend modes, artifact kinds and table scopes are kept by hand in both `lamina-wasm/src/build_dto.rs` and `packages/sdk/src/builder.ts`, some matched by index with no check (reordering one silently mis-tags documents), crossing in two encodings (strings vs `u8`). The render error codes and `HAYRO_VERSION` are also hand-copied into TS. Define each list once in Rust and generate the TS side (as `generated/codes.ts` already is), with one encoding (strings).

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse), [maintainability guideline](../../guidelines/maintainability.md) rule 3; `lamina-wasm/src/build_dto.rs:185-257`, `packages/sdk/src/builder.ts:68-135`, `lamina-build/src/tag/mod.rs:80` (`TagKind::ALL`), `lamina-build/tests/tagging.rs:875` (scrapes TS source today), `packages/render/src/index.ts:153-178`, `packages/render-hayro/src/index.ts:31`, `cargo xtask gen codes`.

## Scope

**In scope**

- Extend `cargo xtask gen` (or add `gen boundary`) to write `packages/sdk/src/generated/builder-enums.ts` and `packages/render/src/generated/codes.ts`; strings on the wire; `HAYRO_VERSION` exported from lamina-render-wasm or checked against `Cargo.toml`; `--check` in `check docs`.

**Out of scope** (and which todo covers it)

- Error shapes ([Q2j](./Q2j-boundary-errors-rust.md), [Q2k](./Q2k-boundary-errors-ts.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Generate, switch the encodings, delete the hand lists and the TS-scraping test.

## Acceptance criteria

- [ ] No hand-kept boundary list remains; `gen --check` fails if one drifts.

## Post-merge checks

None.

## Tests

Generated-file freshness check; existing builder and tagging tests.

## Docs to update

TypeScript guideline (generated code), CLAUDE.md `gen` commands, plan §9.

## Risks and open questions

Changing `u8` indices to strings changes the WASM call signatures: update `@laminapdf/sdk` in the same PR.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
