# Q1f — Examples readers can copy: no scaffolding, `using`, small functions, named numbers

| Field | Value |
| --- | --- |
| ID | `Q1f` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1c](./Q1c-examples-check.md), [Q1d](./Q1d-example-api-cleanups.md) |
| Blocks | [Q1j](./Q1j-recipes-and-troubleshooting.md), [Q1l](./Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `refactor/q1f-examples-readable` |
| PR title | `refactor(examples): examples readers can copy into their apps` |
| Size | M |
| PR | — |

## Goal

Rewrite the TypeScript and Rust examples to the [examples guideline](../../guidelines/examples.md): build options come from the caller (the tests pass the fixed ID, date and producer), self-checks move into the tests, documents and builders are freed with `using`, long functions split into named steps with named constants, headers say what the example shows with no internal references, and TS/Rust pairs share names. The reference PDFs stay byte-identical, proving nothing changed.

## Context

- [Quality plan §2.2](../../plans/quality.md#22-code-examples): every finding with its file.
- `packages/sdk/examples/*.ts`, `crates/lamina/examples/*.rs`, `packages/sdk/test/suite/examples.ts`, `crates/lamina/tests/examples.rs`, `fixtures/golden/examples/`.

## Scope

**In scope**

- `hello(options)`/`build(options)` signatures; one `EXAMPLE_OPTIONS` in the TS suite and in `tests/examples.rs`.
- Move `generate-then-edit`'s byte and title checks and `invoice::builder`'s diagnostics seam into the tests.
- `using` for `Document`/`Builder` in every TS example; no `free()`/`try`-`finally` boilerplate.
- Split `invoice` (both languages), `report`, `images-and-vectors`/`vectors` and `multilingual` into small functions; named layout constants; page width from the API; `report`'s fake data from a named generator; `multilingual.ts` takes `{ font, line, paragraph, direction }[]` instead of parallel arrays.
- `open-untrusted.ts`: named `MiB`, optional `password`.
- Rename for parity: `vectors.rs` → `images_and_vectors.rs` (and its golden), and align header first sentences.

**Out of scope** (and which todo covers it)

- Worker and worker-client examples ([Q1g](./Q1g-worker-examples.md)).
- Quick-start apps ([Q1e](./Q1e-quick-start-apps.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Refactor one pair at a time; after each, run both golden suites.
- [ ] Rename the golden file with `cargo xtask golden --update` only for the rename (contents identical).

## Acceptance criteria

- [ ] Every reference PDF is byte-identical to before (except the renamed file's name).
- [ ] `docs-snippets`, the examples Biome override and `clippy-examples` report nothing for these files.
- [ ] No example is over 100 lines or has a function over 40.

## Post-merge checks

None.

## Tests

The existing golden suites (Rust `tests/examples.rs`, the TS suite in every runtime).

## Docs to update

The guides that show these examples (captions only), plan §9.

## Risks and open questions

Byte-identical output can break if refactoring changes object order (fonts registered in a different order): keep registration order and say so in a comment only where it matters.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
