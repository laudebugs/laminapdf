# Q2m — `@laminapdf/test-utils` for shared TypeScript test helpers

| Field | Value |
| --- | --- |
| ID | `Q2m` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| ADRs | None |
| Branch | `refactor/q2m-ts-test-utils` |
| PR title | `refactor(test): @laminapdf/test-utils for shared TS test helpers` |
| Size | M |
| PR | [#162](https://github.com/laudebugs/lamina/pull/162) |

## Goal

The render adapters import `../render/test/{contract,browser-harness,browser-config}.js` by relative path, which code-organization §2 forbids; the Firefox profile workaround is copied three times; and about 20 test files compute `../../../` to find the repository root. This todo creates a private `@laminapdf/test-utils` for the render contract suite, the browser harness and configuration, the Firefox workaround, the PNG decoder and one repository-root resolver.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/render-{hayro,pdfium}/test/*`, the `vitest.browser.config.ts` files, `packages/render/test/browser-config.ts:17`, `packages/sdk/vitest.browser.config.ts:12`, `packages/viewer/vitest.browser.config.ts:11`; code-organization §2 (names `@laminapdf/test-utils` as its example).

## Scope

**In scope**

- The package (private, workspace-only); the three packages migrated; no relative import into another package; [Q2u](../done/Q2u-ts-dependency-tools.md)'s baseline emptied if it has landed.

**Out of scope** (and which todo covers it)

- Render adapter helpers ([Q2n](../done/Q2n-render-adapter-shared.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Create the package with its own `tsconfig` and `biome.json`.
- [x] Move the contract suite, harness, configuration, Firefox workaround, PNG decoder and root resolver.
- [x] Migrate render, render-hayro, render-pdfium, sdk and viewer tests; run Node and the three browsers.

## Acceptance criteria

- [x] No relative import from one package into another.
- [x] Every suite passes in Node and the three browsers.

## Outcome

`@laminapdf/test-utils` (`packages/test-utils`) holds the shared test helpers, and no package imports another's files by relative path: dependency-cruiser's known-violations list is now empty. Five details add to the plan.

- **TypeScript source, no build:** the package exports its `src/*.ts` files directly (`render-contract`, `browser-harness`, `browser-config`, `firefox`, `root`). The TypeScript guideline names it as the one exception to its packaging rule. Vitest compiles them for the tests, and Node strips their types when a Vitest config imports them, so nothing has to be built before a test run. Node doesn't map a relative `.js` import to its `.ts` file, so the package's modules import each other through the package name (`@laminapdf/test-utils/firefox`), whose `exports` point at the `.ts` files. A Firefox run caught the relative import. It has no `tsconfig.build.json`, which keeps it out of the TypeDoc check, and its `biome.json` turns off the function-length rule for `src/`: everything there is test code, like the `test/` folders the root config already exempts.
- **The Firefox workaround** is `firefoxProvider(packageDir)` in its own `firefox` module. The SDK's and the viewer's browser configs use it, and so does `renderBrowserConfig` for both adapters, so the workaround has one copy instead of three.
- **The repository root** is `fromRoot(path)` in `root`. Six test files that run under Vitest used to count `../` from their own folder and now read files by their path from the root: both adapters' `node.test.ts`, the SDK's `helpers.ts`, `source.test.ts` and `vite.test.ts`, and the WASM smoke test. Of the 19 files that counted `../`, the other 13 keep their paths:
  - Vite's `import.meta.glob` needs literal paths, so the browser tests keep their glob patterns;
  - the Deno, Bun and workerd runners load the SDK's tests their own way, `packages/sdk/bench/reports.mjs` is a plain Node script, and the Worker example (`packages/sdk/examples/worker/`) uses only the public API, so they keep their own paths;
  - `@laminapdf/render`'s own tests keep theirs. `@laminapdf/test-utils` depends on `@laminapdf/render` for the contract suite, and a dependency back made a cycle, so pnpm type-checked test-utils before render was built.
- **Versioning:** the package is in `.changeset/config.json`'s `ignore` list, like the docs site, so it stays at `0.0.0` outside the shared version.
- **Docs:** the package has a README listing what each module offers and what it doesn't reach, and the render README points to it for the contract suite. The maintainability guideline names the package, and CLAUDE.md's layout lists it.

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
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
