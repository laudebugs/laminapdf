# Q2b — TypeScript lint rules: complexity, function length, throws and TSDoc

| Field | Value |
| --- | --- |
| ID | `Q2b` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| ADRs | None |
| Branch | `ci/q2b-ts-lint-rules` |
| PR title | `ci(ts): Biome complexity and throw rules, TypeDoc check for undocumented exports` |
| Size | S |
| PR | [#145](https://github.com/laudebugs/lamina/pull/145) |

## Goal

This todo adds the TypeScript rules from the [maintainability guideline](../../guidelines/maintainability.md) that the tools already in the repository can check: Biome's cognitive complexity (15), function length (60) and `useThrowOnlyError`, and TypeDoc's check for undocumented exports. Today's offenders are listed in an override until [Q2p](../done/Q2p-ts-complexity-splits.md) splits them.

## Context

- The [maintainability guideline](../../guidelines/maintainability.md) ("Size", "Errors"); root `biome.json` and package `biome.json` files; TypeDoc (already a dependency of `apps/docs`).
- Today's offenders (complexity > 15): the PDFium `render` closure (39), `server.ts` `execute` (25), `planTile` (22), `clientCall` (21), two functions in `source/http.ts`, `PdfiumDocument.open`, hayro `fill`.

## Scope

**In scope**

- Root Biome: `noExcessiveCognitiveComplexity` (15), `noExcessiveLinesPerFunction` (60), `useThrowOnlyError`, all as errors, with an override naming today's offenders.
- TypeDoc `validation.notDocumented` with `treatWarningsAsErrors` for each package entry, in `check ts`.

**Out of scope** (and which todo covers it)

- Dependency-cruiser, jscpd and knip ([Q2u](../done/Q2u-ts-dependency-tools.md)).
- Splitting the offenders ([Q2p](../done/Q2p-ts-complexity-splits.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Add the Biome rules; run `biome ci`; list the offenders in an override with a comment pointing at Q2p.
- [x] Fix any `useThrowOnlyError` hits outside the viewer worker, which [Q2k](../done/Q2k-boundary-errors-ts.md) handles (list those in the override too).
- [x] Add the TypeDoc validation run for each package entry; document the missing TSDoc or list it.
- [x] Wire both into `cargo xtask check ts`.

## Acceptance criteria

- [x] `cargo xtask check ts` runs the rules and passes on `main` with the listed exceptions.

## Outcome

All four rules are checked, and four details differ from the plan.

- **Where the offenders are listed:** Biome matches a root override's paths against each nested config's folder, so a path such as `packages/render/src/index.ts` in the root never matches. Each package lists its own offenders in its `biome.json`:
  - `@laminapdf/render`, `@laminapdf/render-hayro` and `@laminapdf/render-pdfium` list `src/index.ts`, for complexity;
  - `@laminapdf/sdk` lists `src/worker/server.ts` and `test/source.test.ts` for complexity, and `src/source/http.ts` for complexity and length;
  - `@laminapdf/viewer` lists `src/worker/document.ts`, for its plain-object throws, which Q2k replaces.

  Each file is exempt only from the rules it breaks today, so a new long function in a file listed for complexity still fails.

  JSON has no comments, so [typescript.md](../../guidelines/typescript.md#lint-format-test) explains the lists and points at Q2p and Q2k.
- **Tests and length:** test files are exempt from the 60-line limit (an override on `**/test/**` and `**/*.test.ts`). A `describe` callback holds a whole suite, and eight test files would otherwise be listed for that alone. The complexity and throw rules apply to tests too.
- **What TypeDoc checks:** the script finds every package under `packages/` with `exports`, and reads its entries from those `exports` (every subpath and condition, such as the SDK's `node` and `workerd` entries), so a new entry is checked without editing it. `@laminapdf/wasm` is left out: wasm-bindgen generates its code and types. `requiredToBeDocumented` covers declarations and public methods and accessors, not every property. With every property included, the viewer alone had 60 warnings, most of them fields inside type literals.
- **No list for TypeDoc:** the check found 18 undocumented exports across the five packages, and this PR documents all 18. No exception list is needed.

## Post-merge checks

None.

## Tests

The tools themselves; a scratch-branch run showing each rule failing (in the PR description).

## Docs to update

typescript.md, the maintainability guideline if anything differs, plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
