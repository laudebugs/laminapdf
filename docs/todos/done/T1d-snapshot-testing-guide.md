# T1d — A docs guide: snapshot-test your own generated PDFs

| Field | Value |
| --- | --- |
| ID | `T1d` |
| Status | `done` |
| Plan | [trust and proof §T1](../../plans/trust-and-proof.md#t1-publish-the-proof) |
| Round / milestone | Round T (trust and proof) |
| Depends on | None |
| ADRs | Follows [0007 §6](../../decisions/0007-save-planner.md#6-sinks-and-determinism) (byte-deterministic output: no clocks, `/ID[1]` from a content hash), [0017](../../decisions/0017-versioning-and-release.md#3-0x-semantics-and-pre-releases) (versions; the stability page counts changed bytes as a breaking change), [0004](../../decisions/0004-render-backend.md#2-adapters-and-default-per-surface) (hayro, the pure-Rust renderer, for drawing pages outside the viewer) |
| Branch | `feat/t1d-snapshot-testing-guide` |
| PR title | `feat(docs): a guide and examples for testing the PDFs you generate against reference PDFs` |
| Size | M (≈ 250–350 lines of examples and tests, plus the guide: a TypeScript and Rust example pair, a TypeScript page-comparison example, their tests and a small change to the snippet check) |
| PR | [#295](https://github.com/laudebugs/laminapdf/pull/295) |

## Goal

Write a guide that shows developers how to test the PDFs their own code makes: fix the file ID and the dates, compare the bytes with a reference PDF they approved, and draw both files' pages to see what changed when the bytes differ. Lamina PDF's output is already byte-identical across runs, runtimes and Rust; this guide turns that property into a test that readers can copy.

Developers on Hacker News complain that generators change their output under them: "every single release changed something in the output." ([17567576](https://news.ycombinator.com/item?id=17567576)). They build pixel comparisons by hand: "the proposed approach of rendering to an image and comparing pixels is best." ([34960757](https://news.ycombinator.com/item?id=34960757)). Talk of deterministic, testable output rose from 9.4 to 16.5 per 1,000 generation comments over the decade ([generation note §1.15](../../research/hacker-news/notes/generation.md), [report R9](../../research/hacker-news/pdf-users-on-hacker-news.md#r9-say-loudly-what-lamina-already-has)).

## Context

- [Report R9](../../research/hacker-news/pdf-users-on-hacker-news.md#r9-say-loudly-what-lamina-already-has) and the [generation note](../../research/hacker-news/notes/generation.md), §1.15 and recommendation 7 ("Sell byte-identical output and test tooling").
- What makes output repeatable, and the API a test uses:
  - `BuildOptions` (`crates/laminapdf-build/src/builder.rs`): `with_id` fixes `/ID[0]` (without it the writer derives one from the content, which is equally repeatable), `with_producer`, and `with_creation_date`, which is given and never read from a clock. `with_compress(false)` keeps content streams readable. `Metadata::with_mod_date` sets the other date.
  - TypeScript's `BuilderOptions` (`packages/sdk/src/builder.ts`): `id` (32 hex digits or 16 bytes), `producer`, `creationDate` (`"D:20261030120000Z"`), `compress`.
  - [ADR 0007 §6](../../decisions/0007-save-planner.md#6-sinks-and-determinism): saves are byte-deterministic too, so an edited file saves the same way twice.
  - The docs site's [stability policy](../../../apps/docs/src/content/docs/reference/stability.mdx): a change to the bytes written for the same input and options is a breaking change, and "the same input gives the same bytes everywhere" holds across versions.
- How Lamina PDF tests its own output, which the guide shows readers how to do for theirs: `crates/laminapdf/tests/examples.rs` and `packages/sdk/test/suite/examples.ts` pass fixed options (`packages/sdk/test/suite/hello-options.ts`'s `exampleOptions`) and compare every example's bytes with its reference PDF in every runtime ([B6c](../done/B6c-runtime-matrix.md)).
- Drawing pages: `@laminapdf/render-hayro`'s `initHayro` and `@laminapdf/render`'s `bytesSource` give a `RenderDocument` whose `render({ page, scale })` returns straight RGBA pixels ([C2c](../done/C2c-render-packages.md)). In Node the module's bytes come from `@laminapdf/wasm/lamina_pdf_render_hayro.wasm`, as `packages/render-hayro/test/` loads them. Both packages are published. Rust apps have the same in the `laminapdf-render` crate's `hayro` feature. The command-line `laminapdf render` ([L4e](../done/L4e-cli-render.md)) isn't released yet.
- The rules the new files must follow:
  - [examples.md](../../guidelines/examples.md): public entry points only, today `@laminapdf/sdk` and `@laminapdf/sdk/worker`; no test scaffolding; ≤ 100 lines; pairs share a name and their first header sentence;
  - [docs-writing.md](../../guidelines/docs-writing.md): an opening of at most 50 words; a sentence before each `<Snippet>`; "reference PDF", never "golden"; no "CI" in reader text.
- What the checks do: `tools/xtask/src/docs_snippets.rs` flags a quoted PDF date (`has_fixed_date`) and a `lamina-ex-` ID as `test-scaffolding`. That rule exists for examples that make documents, but a test example fixes those values on purpose. Pairs are listed in `tools/xtask/docs-snippets.toml`. Biome's `noRestrictedImports` in `packages/sdk/biome.json` names the public entry points in its message. Examples that write no PDF are listed as `READERS` in both `crates/laminapdf/tests/examples.rs` and `tools/xtask/src/examples.rs`, and as `[[example]]` entries in `crates/laminapdf/Cargo.toml` ([Q1c](../done/Q1c-examples-checks.md)).
- The docs site's sidebar (`apps/docs/plugins/sidebar.mjs`): the "Production" group, today only "Keep the page responsive".

## Scope

**In scope**

- **An example pair, `packages/sdk/examples/snapshot-test.ts` and `crates/laminapdf/examples/snapshot_test.rs`**, whose shared first sentence is "Checks that a PDF your code makes is byte for byte the reference PDF you approved." Each holds:
  - the options a test passes in place of the clock and a random ID (TypeScript `TEST_OPTIONS`, Rust `test_options()`): a fixed file ID, producer and creation date, named and explained;
  - a function that compares a PDF's bytes with a reference file and answers "same", "created" or "changed". It writes the reference when none exists yet, for the developer to review and commit. When the bytes differ, it writes the new bytes beside the reference (`<name>.new.pdf`) for review, and with `update` set it replaces the reference. The test runner's assertion stays the reader's, so the function works under Vitest, Jest, `node:test` or `cargo test`;
  - in Rust, a `main` that compares two files given on the command line.
- **A page-comparison example, `packages/sdk/examples/compare-pages.ts`** (TypeScript only), whose first sentence is "Draws each page of two PDFs and counts the pixels that differ, to show where a changed PDF looks different." It draws both files' pages with hayro at a named scale and counts the pixels that differ by more than a named tolerance. It returns, per page, that count and the box around them in page points; a different page count is reported, not thrown.
- **The guide**, `apps/docs/src/content/docs/guides/test-generated-pdfs.mdx`, "Test the PDFs you generate", in the sidebar's Production group after "Keep the page responsive". It covers, in this order:
  - why the bytes repeat: no clock, the same bytes in Rust and every JavaScript runtime, and what a reader's own code may still vary (a date from the clock, data in a different order);
  - the test: the snapshot example in TypeScript and Rust tabs, how to run it the first time, and why to open the reference PDF before committing it;
  - when the bytes change: the page comparison, and what its two outcomes mean. With no differing pixels, the file's structure changed but its pages look the same; with some, the boxes show where. Builds with `compress: false` let a diff tool show the content streams;
  - upgrades: pin the exact version; a change to the bytes is listed first in the changelog; regenerate the references in the upgrade's own change and review the comparison;
  - fonts and images: the subset depends on the font file's exact bytes, so the test keeps its font file in the reader's repository; edited files save the same bytes too.
  - It ends with a link to the stability policy, which states the promise the test relies on.
- **The checks and rules the examples need:**
  - a `[[test-example]]` list in `tools/xtask/docs-snippets.toml` that the `test-scaffolding` rule skips, with the snapshot pair in it, and one sentence in examples.md: an example whose subject is testing a reader's PDFs may fix the file ID and dates;
  - `@laminapdf/render` and `@laminapdf/render-hayro` as public entry points in examples.md and in Biome's message, and as workspace dev-dependencies of `@laminapdf/sdk`;
  - `snapshot_test` in both `READERS` lists and in `Cargo.toml`'s `[[example]]` entries; the pair in `docs-snippets.toml`.

**Out of scope** (and which todo covers it)

- A `laminapdf render` or `laminapdf compare` command → [L4e](../done/L4e-cli-render.md) (render); a compare command is a later todo if readers ask for one.
- Comparing extracted text or structure trees between versions: a later guide, once [T3a](../ready/T3a-text-layer-status.md)–[T3c](../ready/T3c-tables-across-pages.md) settle the extraction output.
- Visual comparison in Rust as an example: the guide names the `laminapdf-render` crate. A Rust example would make `laminapdf_render` a second public import in Rust examples, which needs its own decision.
- Tagged output by default ([trust and proof §9](../../plans/trust-and-proof.md#9-open-questions), question 1): it would change every reader's bytes once, which the guide's upgrade section covers.

## Tasks

- [x] Claim: the first commit moves this file to `ongoing/`.
- [x] `docs_snippets.rs` and `docs-snippets.toml`: the `[[test-example]]` list, with a test that a listed file may hold a quoted PDF date and an unlisted one may not; the pair entry.
- [x] examples.md: the test-example sentence and the two render packages as entry points; Biome's message; the sdk's dev-dependencies.
- [x] `tools/xtask/src/wasm.rs`: build `@laminapdf/render` and `@laminapdf/render-hayro` before `@laminapdf/sdk`'s `test:wasm`. Today they build after it, and the sdk's typecheck (`tsconfig.json` includes `examples/**`) would then fail on `compare-pages.ts`'s imports.
- [x] `snapshot-test.ts` and `snapshot_test.rs` (each ≤ 100 lines, functions ≤ 40 lines, every number named).
- [x] `compare-pages.ts`.
- [x] Tests (below); `READERS`, `Cargo.toml`.
- [x] The guide, drafted with `/writing`; the sidebar entry; a link to it from the stability policy's "The same input gives the same bytes everywhere" item and from the recipes index.
- [x] `/writing-review` on the guide.

## Acceptance criteria

- [x] `packages/sdk/test/snapshot-test.test.ts`, in a temporary folder: the first comparison of `hello(TEST_OPTIONS)` answers "created" and writes the reference; the second answers "same". A build with another producer answers "changed" and writes `<name>.new.pdf` without touching the reference, and with `update` it replaces it.
- [x] `crates/laminapdf/tests/examples.rs` runs the same four cases through `snapshot_test`'s function, with `hello::build(test_options())`.
- [x] `hello` built with the TypeScript `TEST_OPTIONS` and with the Rust `test_options()` gives the same bytes: both tests compare with one SHA-256 constant, declared in the TypeScript test and read by the Rust test at compile time, as `benches/report.rs` reads `REPORT_SHA256`.
- [x] `packages/sdk/test/compare-pages.test.ts`: `hello` against itself has no differing pixel on its page; `hello` against the invoice has some, inside the page's box; a two-page file against a one-page file reports the page count. (The page-count case uses the four-page links-and-outlines reference PDF.)
- [x] `cargo xtask check docs` passes strict: the guide's prose, the snippets (pair, headers, imports, length) and the scaffolding rule with its new list.
- [x] The guide shows code only through `<Snippet>`, and its Markdown twin holds the three files.

## Post-merge checks

None.

## Tests

The two TypeScript tests in `packages/sdk/test/` (Node project); the Rust cases in `crates/laminapdf/tests/examples.rs`; `docs_snippets.rs`'s unit test for the new list; the docs build and its link check. `compare-pages.test.ts` needs `cargo xtask wasm` to have built the hayro module, as the render packages' own tests do.

## Docs to update

- [examples.md](../../guidelines/examples.md) (the test-example sentence, the entry points) and `CLAUDE.md`'s `check docs-snippets` line (the new list).
- `apps/docs/plugins/sidebar.mjs` (the guide); the stability policy and the recipes index (links).

## Risks and open questions

- **Readers' own variation.** A test fails for reasons the reader controls: a date from the clock, a random ID, records in another order, a different font file. The guide lists them, so a failing test points at the reader's code before Lamina PDF.
- **Renderer differences.** hayro draws a small share of pages differently from PDFium ([ADR 0004 §6](../../decisions/0004-render-backend.md#6-when-hayro-becomes-the-default-c1as-criteria)). The comparison draws both files with the same renderer, so those differences cancel out; the guide says so.
- **Import rule.** Adding the two render packages to the examples' entry points widens a guideline. If the owner prefers to keep examples on `@laminapdf/sdk` alone, `compare-pages.ts` moves to `packages/render-hayro/examples/`, and Snippet's glob gains that folder.
- **The scaffolding exemption** must stay narrow: listed files only, each with a reason, so fixed IDs and dates stay out of the examples that make documents.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
