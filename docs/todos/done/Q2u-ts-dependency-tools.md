# Q2u — TypeScript dependency tools: cycles, cross-package imports, duplication, unused exports

| Field | Value |
| --- | --- |
| ID | `Q2u` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2o](./Q2o-ts-hygiene.md) |
| ADRs | None |
| Branch | `ci/q2u-ts-dependency-tools` |
| PR title | `ci(ts): dependency-cruiser, jscpd and knip in check ts` |
| Size | M |
| PR | [#147](https://github.com/laudebugs/lamina/pull/147) |

## Goal

This todo adds three dev tools to `cargo xtask check ts`, after a licence check: dependency-cruiser for import cycles and imports that reach into another package, jscpd for duplicated blocks, and knip for unused exports and files.

## Context

- The [maintainability guideline](../../guidelines/maintainability.md) ("Public surface and tests", "How it's checked"); ADR 0019 and `cargo xtask npm-licenses` (dependency-cruiser MIT, jscpd MIT, knip ISC).
- Today's cross-package imports: `packages/render-{hayro,pdfium}/test/*` and `vitest.browser.config.ts` import `../render/test/…` (fixed by [Q2m](../ongoing/Q2m-ts-test-utils.md)).

## Scope

**In scope**

- dependency-cruiser: no cycles; no relative import into another package; today's render-adapter test imports in a baseline.
- jscpd over `packages/*/src` (at least 10 lines, threshold 0, tests excluded at first).
- knip over the workspace, entry points taken from each package's `exports`.
- All three as root dev dependencies, pinned, through `npm-licenses`.

**Out of scope** (and which todo covers it)

- Biome and TypeDoc rules ([Q2b](../done/Q2b-ts-lint-rules.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Add the three tools and run `cargo xtask npm-licenses`.
- [x] Write each configuration; run each; record today's findings.
- [x] Baseline what other todos fix (Q2m's imports, Q2o's unused exports), fix the rest, and wire all three into `check ts`.

## Acceptance criteria

- [x] `cargo xtask check ts` runs all three and passes on `main` with the recorded baselines.
- [x] Licences pass `npm-licenses`.

## Outcome

All three tools run in `cargo xtask check ts` and pass on `main`. Five details add to the plan.

- **What they found:**
  - dependency-cruiser found the eight render-adapter test imports the Context lists, and no cycles. The eight are its known violations, for Q2m. It cruises `apps/` as well as `packages/`, and `check ts` fails while the list names a violation that no longer occurs, so the list only shrinks.
  - jscpd found one repeated block: the SDK's three entry files (`index.ts`, `entry/node.ts`, `entry/workerd.ts`) bind the same loader functions to their own runtime. That repetition is deliberate. Folding it into a factory would lose the per-runtime doc comments the TypeDoc check requires, so the two per-condition entries are excluded and the maintainability guideline says why.
  - knip found twelve exports used only inside their own file. Five are in `src/`: the viewer worker's `currentBackend`, the SDK's `ExtractGlue`, and the protocol's `Welcome`, `Response` and `Event`. Seven are test helpers: `ssim` in the render contract, and `input`, `buildReport`, `REPORT_PAGES`, `FIXTURE_PATHS`, `MEASUREMENT_PREFIX` and `median` in the SDK's example suite. This PR makes all twelve module-private, so knip has no baseline.
  - For Q2o, knip finds nothing: the viewer's test hooks are reached from tests, which count as uses, and knip doesn't report a package entry's own exports. [Q2o](../done/Q2o-ts-hygiene.md) now says its review, not knip, checks the viewer's exports.
- **TypeScript 7:** dependency-cruiser parses TypeScript through the compiler API, which TypeScript 7 no longer has. Without it, it saw 9 modules instead of 151. `pnpm-workspace.yaml`'s `packageExtensions` gives it TypeScript 6.0.3, the docs site's TypeDoc pin.
- **knip's entries:** `knip.config.ts` computes each package's entries from its `exports`. The mapping of each `dist/**.js` back to its `src/**.ts` lives in `tools/ts/package-entries.mjs`, which the undocumented-export check (Q2b) reads too. It adds what no `exports` can name:
  - the worker files;
  - the test files (`*.test.ts`), but not the helpers they import;
  - the files a test runner loads (Vite's app, workerd's Worker);
  - Vitest configs, examples and benches.

  The generated `@laminapdf/wasm` is left out, and the root workspace has no code of its own. jscpd likewise finds every package's `src/` itself.
- **Versions:** each tool is pinned to a release at least two weeks old when added: dependency-cruiser 18.6.0, jscpd 5.4.1 and knip 6.41.0.
- **How xtask runs them:** they are root `package.json` scripts (`check:deps`, `check:duplicates` and `check:unused`), so knip sees the tools in use and a developer can run one alone.

## Post-merge checks

None.

## Tests

The tools themselves; a scratch-branch run showing each failing (in the PR description).

## Docs to update

typescript.md, the commands list in CLAUDE.md, plan §9.

## Risks and open questions

Tool noise: start each with the narrowest scope that catches real problems, and widen later.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
