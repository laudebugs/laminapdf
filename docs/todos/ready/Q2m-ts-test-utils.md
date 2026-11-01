# Q2m — `@laminapdf/test-utils`: shared TypeScript test helpers

| Field | Value |
| --- | --- |
| ID | `Q2m` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2n](./Q2n-render-adapter-shared.md) |
| ADRs | None |
| Branch | `refactor/q2m-ts-test-utils` |
| PR title | `refactor(test): @laminapdf/test-utils for shared TS test helpers` |
| Size | M |
| PR | — |

## Goal

The render adapters import `../render/test/{contract,browser-harness,browser-config}.js` by relative path (forbidden by code-organization §2), the Firefox profile workaround is copied three times, and about 20 test files compute `../../../` to find the repository root. Create a private `@laminapdf/test-utils` package for the render contract suite, the browser harness and config, the Firefox workaround, the PNG decoder and one repo-root/fixtures resolver.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/render-{hayro,pdfium}/test/*`, `vitest.browser.config.ts` files, `packages/render/test/browser-config.ts:17`, `packages/sdk/vitest.browser.config.ts:12`, `packages/viewer/vitest.browser.config.ts:11`; code-organization §2 (names `@laminapdf/test-utils` as the example).

## Scope

**In scope**

- The package (private, workspace-only); migrate the three packages; remove the relative cross-package imports; dependency-cruiser's baseline from [Q2b](./Q2b-maintainability-checks-ts.md) shrinks to zero.

**Out of scope** (and which todo covers it)

- Render adapter helpers ([Q2n](./Q2n-render-adapter-shared.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Create, move, migrate, run Node and browser suites.

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

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
