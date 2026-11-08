# U6f — Component gallery, screenshot tests, axe on every page and the screens docs check

| Field | Value |
| --- | --- |
| ID | `U6f` |
| Status | `ongoing` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6b](../done/U6b-self-hosted-fonts.md), [U6e](../done/U6e-overlays-and-sheets.md), [U6j](../done/U6j-positioning-and-palette-shell.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md#2-ci-lanes) (CI lanes, flaky tests), [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (`cargo xtask` as the single entry point), 0024 (from [U1a](../done/U1a-adr-0024-app-shell-ux.md)) |
| Branch | `feat/u6f-gallery-and-visual-tests` |
| PR title | `feat(gallery): add the paper component gallery with screenshot and axe tests` |
| Size | M (≈ 350–450 lines of gallery pages, Playwright specs and the xtask check; screenshots don't count) |
| PR | — |

## Goal

Close Round U6 with the guard rails that keep code and design together: a dev-only gallery app that renders every token, icon and component of `@laminapdf/ui` in every state, in each UI theme (Paper light, Paper dark, high contrast and forced colours) and, for page-related components, in each page appearance; Playwright screenshot tests over it with a clear workflow for reviewing and updating images; axe on every gallery page; and a docs check that every screen ID in the screen inventory has a section in its area document and a committed design file. After this, a PR that changes how a component looks shows the change as an image diff, and a screen cannot be added to the inventory without its design.

## Context

- [Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used) (every UI todo links its screens; designs change in the same PR as behaviour) and [§7 risks](../../plans/paper-design.md#7-risks) ("designs drift from code": U6f's screenshots are the mitigation); [§6 U6 acceptance](../../plans/paper-design.md#u6-design-system) ("the gallery renders each component in each theme and screenshot tests guard them").
- [Screen inventory](../../design/paper/screens.md) (IDs such as `F01`, `R05`; the area documents in [plan §4](../../plans/paper-design.md#4-areas); the canvases table at the end, with each area's committed files under `docs/design/paper/<area>/`).
- Design language [§8.6 page appearance vs UI theme](../../design/paper/design-language.md#86-page-appearance-vs-ui-theme), [§8.9 checklist](../../design/paper/design-language.md#89-accessibility-checklist-wcag-22-aa) (forced colours, reduced motion, 200% zoom, 320 px reflow), [§10 authoring notes](../../design/paper/design-language.md#10-authoring-notes-for-the-design-canvas) (artboard file names and `canvas.json`).
- Inputs: [U6a](../done/U6a-paper-tokens.md) tokens, [U6b](../done/U6b-self-hosted-fonts.md) fonts (needed for stable screenshots), [U6c](../done/U6c-icon-set.md) icons, [U6d](../done/U6d-core-components.md) components and its `test/states.ts` helper, [U6e](../done/U6e-overlays-and-sheets.md) overlays.
- [ADR 0018](../../decisions/0018-test-strategy.md) (lanes; [flaky tests](../../decisions/0018-test-strategy.md#4-flaky-tests) are quarantined, not retried silently); [code organization §1](../../guidelines/code-organization.md#1-top-level-layout) (apps are never depended on) and [§6](../../guidelines/code-organization.md#6-tests-and-fixtures) (goldens change only through an explicit command and are reviewed like code).
- [C7a](./C7a-accessibility-and-theming.md) runs axe on the *app's* states; this todo runs it on the components in isolation, so both exist and neither replaces the other.

## Scope

**In scope**

- `apps/gallery` (private, never deployed to production; Vite; depends on `@laminapdf/tokens`, `@laminapdf/fonts`, `@laminapdf/icons`, `@laminapdf/ui`): one route per component group (tokens: colours with their contrast pairs, type scale, spacing, radii, shadows; icons; buttons and toggles; mode switcher and tabs; form controls; swatches and appearance chips; badges, notices, toasts; progress and empty states; status bar items; thumbnails and outline rows; dialogs; sheets; menus and popovers; selection toolbar shell; palette shell), each rendering every state from `test/states.ts`.
- Global controls: theme (light, dark, high contrast), page appearance (Original, Sepia, Dark, High contrast) for page-related routes, density (desktop, touch), direction (LTR, RTL), reduced motion; reflected in the URL so a screenshot can address any combination.
- Playwright screenshot tests (`apps/gallery/e2e/visual.spec.ts`, `toHaveScreenshot`): every route × {light, dark, high contrast} in Chromium at 1× and a touch-density pass at a 390 px viewport; page-related routes × four page appearances; a `forcedColors: "active"` pass per route; one RTL pass per route. Fonts load from `@laminapdf/fonts`; animations disabled; a fixed date and locale.
- Deterministic environment: tests run in the pinned Playwright Docker image (version pinned with the `@playwright/test` version), locally and in CI, so Linux screenshots match on every machine.
- Review and update workflow: `cargo xtask gallery` (build and serve), `cargo xtask gallery --test` (run in the Docker image), `cargo xtask gallery --update` (regenerate screenshots in the Docker image and print the changed files); CI uploads expected, actual and diff images as an artifact on failure; the PR includes the updated PNGs, and the reviewer checks them in GitHub's image diff. Only `--update` writes snapshots; tests never update them.
- axe (via `@axe-core/playwright`) on every route in every theme, in Chromium, Firefox and WebKit: zero violations.
- **Screens docs check**: a `cargo xtask check docs` step (new module `tools/xtask/src/design_screens.rs`) that reads `docs/design/paper/screens.md`, collects every screen ID from its tables, maps each ID to its area document and folder using the ID ranges in the canvases table of screens.md (R01–R09 → reader-a, R10–R18 → reader-b, O01–O08 → edit-session-organise, O09–O17 → pages-navigation-extraction, and so on), and fails when an ID has no heading in its area document that contains the ID, or no committed design file `docs/design/paper/<area>/project/<ID>-*.dc.html`. Foundation screens F01–F05 are documented in screens.md itself and drawn as `foundations/project/{Main,Desktop,Mobile-read,Mobile-select,Style}.dc.html`: the check maps them explicitly and requires no heading. It also checks that every board in each `canvas.json` has a file and vice versa. IDs listed under a "Not yet drawn" heading in screens.md are reported, not failed, so the check can land before every area is drawn.
- CI wiring: a job for the gallery's visual and axe tests in the fast lane when `packages/ui`, `packages/tokens`, `packages/icons`, `packages/fonts` or `apps/gallery` change (via `cargo xtask changes`), and in every scheduled full run.

**Out of scope** (and which todo covers it)

- Full-app states (reader with a document, dialogs in context) and the manual screen-reader pass → [C7a](./C7a-accessibility-and-theming.md); render-backend screenshots of PDF pages → [X3b](../done/X3b-render-diff-nightly.md).
- Deploying the gallery publicly or as a docs-site page → [B7a](../done/B7a-docs-site.md) may embed it later; preview deploys → [C4d](./C4d-deploy-previews-and-production.md) could add it.
- Writing the missing area documents or canvases → the paper-design work and each area's todos; this todo only checks them.
- Screenshots in Firefox and WebKit (their rendering of fonts differs; axe runs there instead): revisit if a cross-engine visual bug slips through.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`. Load the `/rust-best-practices` skill before writing the xtask module.
- [ ] Scaffold `apps/gallery` (Vite, React 19, `"private": true`, nested `biome.json` with DOM globals, `tsconfig.json` with `"module": "preserve"`), import `@laminapdf/tokens/tokens.css` and `@laminapdf/fonts/fonts.css`, preload the UI font.
- [ ] Build the routes from Scope; each route imports the state lists from `@laminapdf/ui`'s test helpers (exported on a `./testing` subpath) instead of repeating them.
- [ ] Add the global controls and URL parameters; an index page listing every route.
- [ ] Add `@playwright/test` and `@axe-core/playwright` (licence-check: Apache-2.0 and MPL-2.0, dev only); `playwright.config.ts` with the projects above; pin the Docker image tag next to the npm version and check they match in a test.
- [ ] `e2e/visual.spec.ts` (screenshots), `e2e/a11y.spec.ts` (axe on every route × theme × engine), `e2e/fonts.spec.ts` (no request leaves the origin: complements [U6b](../done/U6b-self-hosted-fonts.md)).
- [ ] A coverage test: every exported component of `@laminapdf/ui` appears on at least one route (compare the package's export list with a registry in the gallery).
- [ ] `cargo xtask gallery [--test|--update]` in `tools/xtask` (calls `pnpm` and `docker`; a clear error if Docker is missing, with the CI fallback: run the "update screenshots" `workflow_dispatch` job, which uploads the new PNGs as an artifact).
- [ ] `tools/xtask/src/design_screens.rs` with unit tests on small Markdown inputs (ID found, heading missing, file missing, "Not yet drawn" exemption), wired into `check docs`; run it on the real tree and either fix gaps or list them under "Not yet drawn" in screens.md in this PR.
- [ ] CI: the gallery job (Docker image, `pnpm install --frozen-lockfile`, `cargo xtask gallery --test`), artifact upload on failure; the `workflow_dispatch` update job.
- [ ] `apps/gallery/README.md`: how to run, how to update screenshots, how reviewers read image diffs, how to quarantine a flaky screenshot (ADR 0018).

## Acceptance criteria

- [ ] The gallery renders every exported component of `@laminapdf/ui` (coverage test) in every theme, and page-related components in every page appearance.
- [ ] Screenshot tests pass in CI and locally in the Docker image; changing a token value (e.g. `accent`) makes them fail with diff images in the artifact (shown once in the PR description).
- [ ] `cargo xtask gallery --update` regenerates screenshots and nothing else; no test writes snapshots.
- [ ] axe reports zero violations on every route, theme and engine.
- [ ] The gallery makes no request to any origin other than its own (test).
- [ ] `cargo xtask check docs` fails when a screen ID in screens.md has no area-document heading or no committed design file and is not listed under "Not yet drawn" (unit tests), and passes on the tree as merged.
- [ ] Two consecutive CI runs produce identical screenshots (no flakiness from fonts, animation or time).
- [ ] `cargo xtask check` passes.

## Post-merge checks

- [ ] The `workflow_dispatch` "update screenshots" job runs on `main` and uploads an artifact (it needs the workflow on the default branch).

## Tests

`apps/gallery/e2e/visual.spec.ts`, `a11y.spec.ts`, `fonts.spec.ts` (Playwright); the gallery coverage test; `tools/xtask/src/design_screens.rs` unit tests; screenshots in `apps/gallery/e2e/__screenshots__/` (committed, reviewed like goldens).

## Docs to update

- `apps/gallery/README.md` (new).
- `CLAUDE.md`: layout (`apps/gallery`), commands (`cargo xtask gallery [--test|--update]`; `check docs` now includes the screens check).
- [Paper design plan §9](../../plans/paper-design.md#9-to-dos): tick "U6a tokens, U6b fonts, U6c icons, U6d core components, U6e overlays, U6f gallery and visual tests" (all six are merged when this lands); §5 rule 2: mention the screens check and the screenshot diff.
- [UI/UX plan §9](../../plans/ui-ux.md#9-to-dos): tick the U6 line.
- [Screen inventory](../../design/paper/screens.md): the "Not yet drawn" section if any IDs are exempted; [design language §10](../../design/paper/design-language.md#10-authoring-notes-for-the-design-canvas): name the check.

## Risks and open questions

- **Screenshot flakiness** from font hinting, GPU rasterisation and timing: the Docker image, self-hosted fonts, disabled animations and a single engine keep it stable; quarantine per ADR 0018 rather than raising thresholds. Start with `maxDiffPixels: 0` and loosen only with a recorded reason.
- **Repository size**: about 30 routes × several themes is a few hundred PNGs; keep each route's screenshot to its states grid at a fixed width, and consider Git LFS only if the folder passes ~50 MB.
- **Docker on contributors' machines**: not everyone has it; the `workflow_dispatch` update job is the fallback, and the README says so.
- **The screens check before the area documents exist**: area documents and canvases listed in the plan may not all be committed when this lands; the "Not yet drawn" list makes the gap visible instead of blocking the round. Decide with the owner whether that list must be empty before any Round C UI todo merges.
- **Gallery in `apps/` vs a route in `apps/web`**: chosen as its own app so it doesn't wait for [C4b](./C4b-web-app-shell.md) and never ships to users; it may move into the docs site later.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
