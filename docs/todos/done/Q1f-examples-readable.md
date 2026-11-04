# Q1f — Examples readers can copy

| Field | Value |
| --- | --- |
| ID | `Q1f` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1c](./Q1c-examples-checks.md), [Q1d](./Q1d-example-api-cleanups.md) |
| Blocks | [Q1j](../ready/Q1j-recipes-and-troubleshooting.md), [Q1l](../ready/Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `feat/q1d-example-api-cleanups` (with [Q1d](./Q1d-example-api-cleanups.md): editing an example makes its checks strict, so the two land together) |
| PR title | `refactor(examples): examples readers can copy into their apps` |
| Size | M |
| PR | [#192](https://github.com/laudebugs/laminapdf/pull/192) |

## Goal

This todo rewrites the TypeScript and Rust examples to the [examples guideline](../../guidelines/examples.md): the caller supplies build options, self-checks move into the tests, `using` frees documents, long functions become named steps with named numbers, headers say what the example shows, and pairs share names. The reference PDFs stay byte-identical, which proves nothing changed.

## Context

- [Quality plan §2.2](../../plans/quality.md#22-code-examples): every finding with its file.
- `packages/sdk/examples/*.ts`, `crates/laminapdf/examples/*.rs`, `packages/sdk/test/suite/examples.ts`, `crates/laminapdf/tests/examples.rs`, `fixtures/golden/examples/`.

## Scope

**In scope**

- `hello(options)` and `build(options)` signatures; one `EXAMPLE_OPTIONS` in the TS suite and one in `tests/examples.rs`.
- `generate-then-edit`'s byte and title checks and `invoice::builder`'s diagnostics seam move into the tests.
- `using` for `Document` and `Builder` in every TS example.
- `invoice` (both languages), `report`, `images-and-vectors`/`vectors` and `multilingual` split into small functions with named layout constants; the page width comes from the API; `report`'s fake data comes from a named generator; `multilingual.ts` takes `{ font, line, paragraph, direction }[]`.
- `open-untrusted.ts`: a named `MiB` and an optional `password`.
- Parity: `vectors.rs` becomes `images_and_vectors.rs` (and its reference PDF is renamed), and pair headers share a first sentence.

**Out of scope** (and which todo covers it)

- Cloudflare Worker and Web Worker examples ([Q1g](../ongoing/Q1g-worker-examples.md)).
- Quick-start apps ([Q1e](../done/Q1e-quick-start-apps.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Introduce `EXAMPLE_OPTIONS` in both test suites and change each example to take options. (One options function per suite instead, since each reference PDF has its own file ID.)
- [x] Move self-checks and seams into the tests.
- [x] Refactor one TS/Rust pair at a time; after each, run both reference suites.
- [x] Add `using` to every TS example and remove the manual freeing.
- [x] Rename `vectors.rs` and its reference PDF (`cargo xtask golden --update` for the rename only), and align header sentences.
- [x] Run `docs-snippets`, the Biome override and `clippy-examples` with `--all-strict` on the examples. (Strict on every example this PR changed; `--all-strict` still reports the Worker examples, which are [Q1g](../ongoing/Q1g-worker-examples.md)'s, and the `text-facts` tool.)

## Acceptance criteria

- [x] Every reference PDF is byte-identical (only the renamed file's name changes), except `invoice.pdf` (see Outcome).
- [x] The three examples checks report nothing on these files.
- [x] No example is over 100 lines or has a function over 40.

## Post-merge checks

None.

## Tests

The reference suites (Rust `tests/examples.rs`; the TS suite in every runtime).

## Docs to update

Captions on the pages that show these examples, plan §9.

## Risks and open questions

Refactoring can reorder font registration and change object order: keep the order, and comment it only where it matters.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome

- Every example takes what an app supplies as parameters: its input files, and `BuildOptions` (TS `BuilderOptions`) where a file ID or date was fixed. The PDF/A and PDF/UA examples have content-derived IDs, so they take only their files. `crates/laminapdf/tests/examples.rs` and the TS suite pass the fixed values. `cargo xtask examples` and `golden --update` build the reference PDFs through the test's ignored `write_every_example`, because the binaries' `main` no longer fixes anything.
- `generate-then-edit`'s self-checks and `invoice::builder` moved into the tests. The TS examples use `using`; functions are named steps; layout numbers are named or come from the page size; `vectors` is `images_and_vectors`; every pair shares its header's first sentence.
- `invoice.pdf` is the one reference PDF that changed. To fit 100 lines with named numbers, the invoice lost its accent bar, payment link and row rules, and its total moved into the amount column. `pdf_ua_report` and `pdf_a_archive` still show links and decoration marked as artifacts. Its render goldens, the page-text snapshot and PDFium's character boxes were regenerated.
- The 100-page report now takes its right edge from the page size, so `REPORT_SHA256` changed, along with its Rust twin, the `report_100_pages` bench.
- `multilingual` takes its fonts as a parameter, and the five scripts' text stays in the example, because that text is what the example shows. Each suite has an options function, not one `EXAMPLE_OPTIONS`, because each reference PDF has its own file ID.
- The Cloudflare Worker example and `worker-client.ts` are left to [Q1g](../ongoing/Q1g-worker-examples.md).
