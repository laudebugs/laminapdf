# Q2b — TypeScript lint rules: complexity, function length, throws and TSDoc

| Field | Value |
| --- | --- |
| ID | `Q2b` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2p](./Q2p-ts-complexity-splits.md) |
| ADRs | None |
| Branch | `ci/q2b-ts-lint-rules` |
| PR title | `ci(ts): Biome complexity and throw rules, TypeDoc check for undocumented exports` |
| Size | S |
| PR | — |

## Goal

This todo adds the TypeScript rules from the [maintainability guideline](../../guidelines/maintainability.md) that the tools already in the repository can check: Biome's cognitive complexity (15), function length (60) and `useThrowOnlyError`, and TypeDoc's check for undocumented exports. Today's offenders are listed in an override until [Q2p](./Q2p-ts-complexity-splits.md) splits them.

## Context

- The [maintainability guideline](../../guidelines/maintainability.md) ("Size", "Errors"); root `biome.json` and package `biome.json` files; TypeDoc (already a dependency of `apps/docs`).
- Today's offenders (complexity > 15): the PDFium `render` closure (39), `server.ts` `execute` (25), `planTile` (22), `clientCall` (21), two functions in `source/http.ts`, `PdfiumDocument.open`, hayro `fill`.

## Scope

**In scope**

- Root Biome: `noExcessiveCognitiveComplexity` (15), `noExcessiveLinesPerFunction` (60), `useThrowOnlyError`, all as errors, with an override naming today's offenders.
- TypeDoc `validation.notDocumented` with `treatWarningsAsErrors` for each package entry, in `check ts`.

**Out of scope** (and which todo covers it)

- Dependency-cruiser, jscpd and knip ([Q2u](./Q2u-ts-dependency-tools.md)).
- Splitting the offenders ([Q2p](./Q2p-ts-complexity-splits.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Add the Biome rules; run `biome ci`; list the offenders in an override with a comment pointing at Q2p.
- [ ] Fix any `useThrowOnlyError` hits outside the viewer worker, which [Q2k](./Q2k-boundary-errors-ts.md) handles (list those in the override too).
- [ ] Add the TypeDoc validation run for each package entry; document the missing TSDoc or list it.
- [ ] Wire both into `cargo xtask check ts`.

## Acceptance criteria

- [ ] `cargo xtask check ts` runs the rules and passes on `main` with the listed exceptions.

## Post-merge checks

None.

## Tests

The tools themselves; a scratch-branch run showing each rule failing (in the PR description).

## Docs to update

typescript.md, the maintainability guideline if anything differs, plan §9.

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
