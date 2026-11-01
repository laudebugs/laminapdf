# Q1f — Examples readers can copy

| Field | Value |
| --- | --- |
| ID | `Q1f` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1c](../done/Q1c-examples-checks.md), [Q1d](./Q1d-example-api-cleanups.md) |
| Blocks | [Q1j](./Q1j-recipes-and-troubleshooting.md), [Q1l](./Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `refactor/q1f-examples-readable` |
| PR title | `refactor(examples): examples readers can copy into their apps` |
| Size | M |
| PR | — |

## Goal

This todo rewrites the TypeScript and Rust examples to the [examples guideline](../../guidelines/examples.md): the caller supplies build options, self-checks move into the tests, `using` frees documents, long functions become named steps with named numbers, headers say what the example shows, and pairs share names. The reference PDFs stay byte-identical, which proves nothing changed.

## Context

- [Quality plan §2.2](../../plans/quality.md#22-code-examples): every finding with its file.
- `packages/sdk/examples/*.ts`, `crates/lamina/examples/*.rs`, `packages/sdk/test/suite/examples.ts`, `crates/lamina/tests/examples.rs`, `fixtures/golden/examples/`.

## Scope

**In scope**

- `hello(options)` and `build(options)` signatures; one `EXAMPLE_OPTIONS` in the TS suite and one in `tests/examples.rs`.
- `generate-then-edit`'s byte and title checks and `invoice::builder`'s diagnostics seam move into the tests.
- `using` for `Document` and `Builder` in every TS example.
- `invoice` (both languages), `report`, `images-and-vectors`/`vectors` and `multilingual` split into small functions with named layout constants; the page width comes from the API; `report`'s fake data comes from a named generator; `multilingual.ts` takes `{ font, line, paragraph, direction }[]`.
- `open-untrusted.ts`: a named `MiB` and an optional `password`.
- Parity: `vectors.rs` becomes `images_and_vectors.rs` (and its reference PDF is renamed), and pair headers share a first sentence.

**Out of scope** (and which todo covers it)

- Cloudflare Worker and Web Worker examples ([Q1g](./Q1g-worker-examples.md)).
- Quick-start apps ([Q1e](../ongoing/Q1e-quick-start-apps.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Introduce `EXAMPLE_OPTIONS` in both test suites and change each example to take options.
- [ ] Move self-checks and seams into the tests.
- [ ] Refactor one TS/Rust pair at a time; after each, run both reference suites.
- [ ] Add `using` to every TS example and remove the manual freeing.
- [ ] Rename `vectors.rs` and its reference PDF (`cargo xtask golden --update` for the rename only), and align header sentences.
- [ ] Run `docs-snippets`, the Biome override and `clippy-examples` with `--all-strict` on the examples.

## Acceptance criteria

- [ ] Every reference PDF is byte-identical (only the renamed file's name changes).
- [ ] The three examples checks report nothing on these files.
- [ ] No example is over 100 lines or has a function over 40.

## Post-merge checks

None.

## Tests

The reference suites (Rust `tests/examples.rs`; the TS suite in every runtime).

## Docs to update

Captions on the pages that show these examples, plan §9.

## Risks and open questions

Refactoring can reorder font registration and change object order: keep the order, and comment it only where it matters.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
