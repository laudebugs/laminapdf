# Q2m — `@laminapdf/test-utils` for shared TypeScript test helpers

| Field | Value |
| --- | --- |
| ID | `Q2m` |
| Status | `ongoing` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2n](../ready/Q2n-render-adapter-shared.md) |
| ADRs | None |
| Branch | `refactor/q2m-ts-test-utils` |
| PR title | `refactor(test): @laminapdf/test-utils for shared TS test helpers` |
| Size | M |
| PR | — |

## Goal

The render adapters import `../render/test/{contract,browser-harness,browser-config}.js` by relative path, which code-organization §2 forbids; the Firefox profile workaround is copied three times; and about 20 test files compute `../../../` to find the repository root. This todo creates a private `@laminapdf/test-utils` for the render contract suite, the browser harness and configuration, the Firefox workaround, the PNG decoder and one repository-root resolver.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/render-{hayro,pdfium}/test/*`, the `vitest.browser.config.ts` files, `packages/render/test/browser-config.ts:17`, `packages/sdk/vitest.browser.config.ts:12`, `packages/viewer/vitest.browser.config.ts:11`; code-organization §2 (names `@laminapdf/test-utils` as its example).

## Scope

**In scope**

- The package (private, workspace-only); the three packages migrated; no relative import into another package; [Q2u](../done/Q2u-ts-dependency-tools.md)'s baseline emptied if it has landed.

**Out of scope** (and which todo covers it)

- Render adapter helpers ([Q2n](../ready/Q2n-render-adapter-shared.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Create the package with its own `tsconfig` and `biome.json`.
- [ ] Move the contract suite, harness, configuration, Firefox workaround, PNG decoder and root resolver.
- [ ] Migrate render, render-hayro, render-pdfium, sdk and viewer tests; run Node and the three browsers.

## Acceptance criteria

- [ ] No relative import from one package into another.
- [ ] Every suite passes in Node and the three browsers.

## Post-merge checks

None.

## Tests

Existing suites.

## Docs to update

CLAUDE.md layout, plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
