# Q2h — One ToUnicode writer and one FontDescriptor builder in lamina-text (fixes split surrogates)

| Field | Value |
| --- | --- |
| ID | `Q2h` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2r](./Q2r-split-large-rust-modules-surface.md) |
| ADRs | None |
| Branch | `fix/q2h-text-writers` |
| PR title | `fix(text): one ToUnicode writer and one FontDescriptor builder` |
| Size | S |
| PR | — |

## Goal

lamina-text has two ToUnicode CMap writers (`simple/tounicode.rs`, `embed/tounicode.rs`) that have drifted: the simple-font one caps text with `encode_utf16().take(MAX_DEST_UNITS)` and can split a surrogate pair. FontDescriptor building is also written twice, differing on `CapHeight` without saying why, and two "number to `Object`" helpers round differently. Make one writer with a codespace parameter, one descriptor builder, and number helpers in lamina-cos.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina-text/src/{simple/tounicode.rs:36, embed/tounicode.rs:42, embed/type0.rs:62-82,206, simple/mod.rs:184-202, embed/subset.rs:278}`, `lamina-build/src/page.rs:334`.

## Scope

**In scope**

- `tounicode` module with `Codespace::{OneByte, TwoByte}`; `descriptor(info, base_font, bbox, kind)` with a comment on `CapHeight`; `Object::number(f64)` and `Object::number_2dp(f64)` (or one with a precision argument) in lamina-cos/lamina-syntax; migrate callers.

**Out of scope** (and which todo covers it)

- Lexer and PNG filter reuse ([Q2i](./Q2i-one-lexer-one-png-filter.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Merge, migrate, test; regenerate no goldens unless the surrogate fix changes one (then explain it).

## Acceptance criteria

- [ ] A 257-unit text ending in an astral character never splits its surrogate pair (new test).
- [ ] One writer, one descriptor builder.

## Post-merge checks

None.

## Tests

New regression test; text goldens and extraction facts.

## Docs to update

Plan §9.

## Risks and open questions

A golden may change if it relied on the split; the facts tests show whether extractors read it better.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
