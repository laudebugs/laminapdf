# U6a — `@laminapdf/tokens`: Paper design tokens, themes, page appearances and a contrast test

| Field | Value |
| --- | --- |
| ID | `U6a` |
| Status | `ready` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U1a](./U1a-adr-0024-app-shell-ux.md) |
| Blocks | [U6b](./U6b-self-hosted-fonts.md), [U6c](./U6c-icon-set.md), [U6d](./U6d-core-components.md) |
| ADRs | Follows 0024 (token layers: primitive → semantic → component; page appearance separate from UI theme; from [U1a](./U1a-adr-0024-app-shell-ux.md)), [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (pnpm workspace, `catalog:`), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (licence check for any new dev dependency) |
| Branch | `feat/u6a-paper-tokens` |
| PR title | `feat(tokens): add @laminapdf/tokens with paper light, dark and high-contrast themes` |
| Size | M (≈ 300–400 lines of generator, tests and config; the token JSON and the generated CSS/TS don't count) |
| PR | — |

## Goal

Turn the Paper design language's colours, type, spacing, shape, depth and layout sizes into one machine-readable source, `packages/tokens/tokens.json`, and generate from it the CSS custom properties (`--paper-<name>`) and typed TypeScript constants that every UI package and app imports. It ships three UI themes (Paper light, Paper dark, high contrast) with four accent colours (Teal, Blue, Plum, Green) for light and dark, the four page appearances (Original, Sepia, Dark, High contrast) as their own set, separate from the UI theme, and annotation colours that are the same in every theme. A contrast test in CI checks every declared text/background and UI-boundary pair against WCAG 2.2 AA, so no later PR can ship an unreadable pair. Before merge the project owner signs off the values the design language marks **(proposed)**; from then on the tokens file, not the Markdown, is the source of truth. Every other U6 todo, C4a's chrome and C7a's themes build on it, so it comes first.

## Context

- [Paper design plan §5 rule 3](../../plans/paper-design.md#5-how-the-designs-are-used) — "tokens come from one place": this todo creates that place; [§6](../../plans/paper-design.md#6-round-u6-paper-in-code) and [§8 open question 1](../../plans/paper-design.md#8-open-questions) — owner sign-off on proposed values happens here.
- Design language: [§2 Colour](../../design/paper/design-language.md#2-colour) (light theme [§2.1](../../design/paper/design-language.md#21-light-ui-theme-default), annotation colours [§2.2](../../design/paper/design-language.md#22-highlight-and-annotation-colours), dark theme [§2.3](../../design/paper/design-language.md#23-dark-ui-theme-proposed), page appearance [§2.4](../../design/paper/design-language.md#24-page-appearance)), [§3 Typography](../../design/paper/design-language.md#3-typography) and the [type scale](../../design/paper/design-language.md#type-scale), [§4 Space, shape, depth](../../design/paper/design-language.md#4-space-shape-depth), [§6 Layout](../../design/paper/design-language.md#6-layout), [§7.0 shared states](../../design/paper/design-language.md#70-shared-states) (hover, pressed, disabled, focus), [§8.6 page appearance vs UI theme](../../design/paper/design-language.md#86-page-appearance-vs-ui-theme), [§8.9 accessibility checklist](../../design/paper/design-language.md#89-accessibility-checklist-wcag-22-aa). Its §2 tables already list the contrast of each pair; the test recomputes them.
- [UI/UX plan §5.5](../../plans/ui-ux.md#55-page-appearance-and-themes-r8) — page modes vs UI themes, annotation colours never change, the gutter has its own token; [§4 principles](../../plans/ui-ux.md#4-principles) 7 and 9.
- ADR 0024 (written in [U1a](./U1a-adr-0024-app-shell-ux.md)) — the token layers and naming this package implements; if it and the design language disagree on names, the ADR wins and the design language is corrected in this PR.
- [C4a](./C4a-ui-chrome-and-platform.md) planned a `src/styles/tokens.css` in `@laminapdf/ui`; this package replaces it (C4a imports `@laminapdf/tokens`). [C7a](./C7a-accessibility-and-theming.md) planned a token contrast script and the dark and high-contrast themes; the values and the contrast test now live here, and C7a wires the theme switch, `forced-colors` styles and settings.
- [U2c](./U2c-page-appearance-modes.md) renders page appearances in both backends; it reads paper, ink and selection colours from this package.
- [TypeScript guidelines](../../guidelines/typescript.md#modules-and-packaging) (ESM-only, named exports, `exports` map); [code organization §9 generated code](../../guidelines/code-organization.md#9-generated-code) (header, no-diff check).
- WCAG 2.2 SC 1.4.3 (contrast minimum, 4.5:1 text), 1.4.11 (non-text contrast, 3:1 for UI component boundaries and focus indicators) and the relative-luminance formula in the WCAG 2.2 definitions (W3C Recommendation).

## Scope

**In scope**

- `packages/tokens` (`@laminapdf/tokens`, private, `"sideEffects": ["*.css"]`): `tokens.json` (source), a generator script, generated `src/generated/tokens.css` and `src/generated/tokens.ts` (committed, `@generated` header, `linguist-generated` in `.gitattributes`), `src/index.ts` exporting the typed constants by name, and an `exports` map with `"./tokens.css"`.
- Source format: the W3C Design Tokens Community Group format (`$value`, `$type`, `$description`, aliases as `{group.name}`; check the current published version of the spec), with lamina metadata under `$extensions["dev.lamina"]`: `status` (`"from-files" | "proposed" | "approved"`) and `contrast` pairs.
- Layers per ADR 0024: **primitive** (raw palette and scales: `color.warm.*`, `color.teal.*`, `space.*`, `radius.*`, `font.size.*` …, never used by components directly) → **semantic** (the design language's names: `ground`, `surface`, `raised`, `field`, `gutter`, `text`, `text-2`, `text-3`, `muted`, `hairline`, `chip-border`, `control-border`, `accent`, `accent-strong`, `on-accent`, `accent-tint`, `selection-tint`, `focus`, `danger`, `positive`, `warning`, the three status tints, `hover`, `pressed`, `disabled`, `scrim`, `pill`) → **component** (only the ones the design language fixes: focus ring width/offsets, button heights and paddings, primary pressed per accent (Teal `#123A3F`, §2.8), swatch sizes, top bar / secondary toolbar / status bar / rail / inspector sizes, phone bar sizes, sheet radius and grabber, thumbnail sizes).
- Themes: **Paper light** (default, [§2.1](../../design/paper/design-language.md#21-light-ui-theme-default)), **Paper dark** ([§2.3](../../design/paper/design-language.md#23-dark-ui-theme-proposed), including `raised` and the dark page shadow), **high contrast** (semantic tokens mapped to CSS system colours `Canvas`, `CanvasText`, `ButtonFace`, `ButtonText`, `ButtonBorder`, `Highlight`, `HighlightText`, `LinkText`, `GrayText`, as the design language says: no hand-made palette). **Accents** ([§2.8](../../design/paper/design-language.md#28-accent-colours-proposed)): `accent`, `accent-strong`, primary pressed, `accent-tint`, `selection-tint` (per page appearance), `field-tint` and `focus` for Teal (default), Blue, Plum and Green in light and dark, under `[data-accent="teal|blue|plum|green"]`, with each accent's dark values emitted for `[data-theme="dark"][data-accent=…]` and inside `@media (prefers-color-scheme: dark)` for `:root:not([data-theme])[data-accent=…]` (Match system); high contrast ignores the accent. Primary pressed is per accent (§2.8), not the component constant below. CSS selectors: `:root` and `[data-theme="light"]`; `[data-theme="dark"]`, plus `@media (prefers-color-scheme: dark)` on `:root:not([data-theme])`; `[data-theme="high-contrast"]`, plus `@media (forced-colors: active)`.
- **Page appearance** tokens, a separate axis from the theme (`[data-page-appearance="original|sepia|dark|high-contrast"]` on the canvas, and as TS constants for the renderer): paper, ink, selection tint per appearance ([§2.4](../../design/paper/design-language.md#24-page-appearance)), plus `gutter` and the page shadows per UI theme.
- **Annotation colours** (yellow, green, blue, pink, [§2.2](../../design/paper/design-language.md#22-highlight-and-annotation-colours)) and the selected-swatch outline: emitted once, never overridden by a theme or appearance block (tested).
- Non-colour tokens: font families with fallbacks (`'Source Sans 3', 'Segoe UI', system-ui, sans-serif`; `Newsreader, Georgia, serif`), the type scale (size / line height / weight / letter spacing per role), spacing scale `4 8 12 16 24 32 48` plus the in-component values the design language allows, radii, border widths, shadows, breakpoints (600, 1024), motion durations (toast 4 s, page pill fade, indeterminate progress) with a `prefers-reduced-motion` override to 0.
- **Contrast test** over every declared pair, run in CI (`cargo xtask check ts` runs package tests): text pairs ≥ 4.5:1, UI boundaries, focus rings and state indicators ≥ 3:1; translucent tokens (`hover`, `pressed`, `scrim`, `pill`) composited over the background they sit on before measuring; pairs the design language marks ⚠ (decorative `hairline`, `chip-border`, selection and highlight fills on white, `disabled`) declared with `"decorative": true` or `"exempt": "<reason>"` and reported, not failed.
- A coverage test: every semantic text token has at least one declared pair with every background token it may sit on in that theme (the list of allowed backgrounds per text token is in the JSON), so a new token cannot skip the contrast test.
- Owner sign-off on every **(proposed)** value, recorded in the JSON (`status: "approved"`) and in the design language.

**Out of scope** (and which todo covers it)

- Fonts themselves (files, `@font-face`, preload) → [U6b](./U6b-self-hosted-fonts.md); this package only names the families.
- Icons → [U6c](./U6c-icon-set.md). Z-order tokens → [U6e](./U6e-overlays-and-sheets.md) (added to this package there, with the overlays that use them).
- Components using the tokens → [U6d](./U6d-core-components.md), [U6e](./U6e-overlays-and-sheets.md); screenshots of every theme → [U6f](./U6f-gallery-and-visual-tests.md).
- The theme switch UI, `forced-colors` component styles, stored user preference and the settings dialog → [C7a](./C7a-accessibility-and-theming.md).
- Recolouring page content in the renderers (Sepia, Dark, High contrast) → [U2c](./U2c-page-appearance-modes.md); page-appearance chips in View settings → [C7a](./C7a-accessibility-and-theming.md) / [U2c](./U2c-page-appearance-modes.md).
- Mockup-only document colours ([§2.5](../../design/paper/design-language.md#25-mockup-only-colours-document-content)) and the drawn macOS `window` colours: not product tokens.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main` (`git worktree add -b feat/u6a-paper-tokens feat-u6a-paper-tokens main`), `pnpm install`.
- [ ] **Ask for sign-off first** (it gates merge, so start it on day one): open a short issue or PR comment listing every **(proposed)** value in the design language, grouped (dark theme; `warning` and the three tints; `hover`/`pressed`/`disabled`; `accent-strong` for buttons; dialog/menu/toast shadows; breakpoints and rail rule; sheet large detent; page pill fade; page-appearance selection tints; the dark page shadow; component styles of §7.18–§7.23), each with its value and contrast. Link it from the PR description.
- [ ] Re-read ADR 0024 as merged; note its layer names and any naming rule (e.g. `--paper-` prefix) and follow it.
- [ ] Scaffold `packages/tokens`: `package.json` (`"type": "module"`, `"private": true`, `exports` with `types` first and `"./tokens.css"`, `"files"`, `"engines"`, scripts `generate`, `typecheck`, `test`), `tsconfig.json` extending `../../tsconfig.base.json`, `README.md`. No runtime dependencies.
- [ ] Write `tokens.json`: primitives first, then semantic tokens per theme (`theme.light`, `theme.dark`, `theme.high-contrast`), page appearances, annotation colours, then component tokens; copy every value from the design language exactly, with `$description` = the role column and `status` from the document (*from the files* vs **(proposed)**).
- [ ] Declare contrast pairs under each text and boundary token (`{ "on": "{surface}", "min": 4.5 }`, `{ "on": "{ground}", "min": 3, "kind": "boundary" }`, `{ "decorative": true }` …), at least every pair the design language's §2 tables list, plus page ink on each paper and on each selection tint, `on-accent` on `accent` and on `accent-strong`, white on primary pressed, `danger` on `danger-tint` etc.
- [ ] Generator `scripts/generate.ts` (hand-written, ≈ 150 lines, no Style Dictionary unless the hand-written one gets unwieldy; if you add it, check its licence, Apache-2.0): resolves aliases, refuses cycles and unknown references, emits `tokens.css` (one block per theme and appearance, primitives not emitted as custom properties unless a semantic token needs them) and `tokens.ts` (`as const` objects and literal-union types such as `ThemeName`, `PageAppearance`, `AnnotationColour`, `SemanticColour`). `--check` mode exits non-zero when the committed output differs.
- [ ] `src/index.ts`: named exports only (`THEMES`, `PAGE_APPEARANCES`, `ANNOTATION_COLOURS`, `SPACE`, `RADIUS`, `TYPE`, `BREAKPOINTS`, `MOTION`, `cssVar(name)` returning `var(--paper-…)` typed by `SemanticColour | …`), each with TSDoc.
- [ ] `src/contrast.ts`: WCAG 2.x relative luminance and ratio, alpha compositing over an opaque background; unit-tested against known values (black/white 21:1, `#767676` on white 4.54:1) and a few design-language numbers.
- [ ] `src/contrast.test.ts`: iterates every declared pair in every theme and appearance; fails with a table of failing pairs (token, background, ratio, required).
- [ ] `src/high-contrast.browser.test.ts` (Vitest browser mode, Chromium, Firefox, WebKit): applies `[data-theme="high-contrast"]`, reads computed system colours with `getComputedStyle`, and checks the same pairs; forced-colors rendering itself is screenshot-tested in [U6f](./U6f-gallery-and-visual-tests.md).
- [ ] Tests that annotation colours are identical in every theme and appearance block, that every semantic token exists in every theme (no missing key falls back silently), and that no component token points at a primitive directly when a semantic one exists.
- [ ] Wire `pnpm --filter @laminapdf/tokens generate --check` into the package's `test` script so `cargo xtask check ts` catches stale output; add the generated paths to `.gitattributes`.
- [ ] Apply the owner's answers: approved values get `status: "approved"`; changed values are changed in the JSON, the design language and (noted for the next canvas edit) the canvases; refused values are replaced before merge.
- [ ] Update the design language (see Docs to update) and the package README (layers, how to add a token, how to declare a pair, the generated table of pairs and ratios).
- [ ] Switch the docs site to `@laminapdf/tokens` (it themes Starlight with an interim copy from [U6g](../done/U6g-docs-site-paper.md)) and delete its `--paper-*` block in `apps/docs/src/styles/paper.css`.

## Acceptance criteria

- [ ] `@laminapdf/tokens` exists with `tokens.json`, committed generated CSS and TS, and `generate --check` passing in CI.
- [ ] Every value in design-language [§2](../../design/paper/design-language.md#2-colour)–[§4](../../design/paper/design-language.md#4-space-shape-depth) (except §2.5 and `window`) has a token; a test lists any colour hex in §2.1–§2.4 and §2.8 that is missing from `tokens.json` (parsed from the Markdown tables).
- [ ] The contrast test passes for Paper light and Paper dark with each of the four accents (8 combinations) and the four page appearances; every text pair ≥ 4.5:1 and every boundary/focus/state pair ≥ 3:1, apart from pairs declared decorative or exempt with a reason; the high-contrast browser test passes in three engines.
- [ ] Changing any text token to a failing value (e.g. `muted` to `#8A8174`) makes CI fail (shown once in the PR description).
- [ ] Annotation colours are byte-identical across all theme and appearance outputs (test).
- [ ] No token in `tokens.json` has `status: "proposed"` (test); the owner's sign-off is linked in the PR.
- [ ] `cargo xtask check` passes, including `npm-licenses`.

## Post-merge checks

None.

## Tests

`packages/tokens/src/*.test.ts` (Vitest, Node): contrast maths, every declared pair, coverage of text tokens × backgrounds, annotation invariance, theme completeness, no `proposed` status, Markdown-to-JSON coverage of §2 hexes. `packages/tokens/src/high-contrast.browser.test.ts` (Vitest browser mode, three engines). Generator `--check` as part of `test`. No fixtures.

## Docs to update

- [Design language](../../design/paper/design-language.md): the header's **Source of truth** line and the intro to §2–§4 say that values now come from `packages/tokens/tokens.json` (linked) and that the tables are a readable copy; remove the **(proposed)** markers for approved values (or note the approval date); fix any ratio the test computed differently.
- [Paper design plan §9](../../plans/paper-design.md#9-to-dos): tick "Owner sign-off on proposed values (U6a)"; leave the combined U6 line for [U6f](./U6f-gallery-and-visual-tests.md); update §3 ("Values marked (proposed) … need sign-off in U6a") to say they were signed off.
- `packages/tokens/README.md` (new). `CLAUDE.md` repository layout: add `packages/` with `tokens/` (the first package, if no other package has landed yet).
- `docs/guidelines/typescript.md` only if the package needs a rule exception (for example CSS side effects).

## Risks and open questions

- **Owner availability gates merge.** Ask on the first day; if some answers take long, land approved values and keep the PR open rather than merging `proposed` ones (the "no proposed status" test makes that explicit).
- **High contrast** is a mapping to system colours, so its contrast depends on the user's OS palette; we can only test the browsers' defaults in normal mode. The `forced-colors` screenshots in [U6f](./U6f-gallery-and-visual-tests.md) and C7a's manual pass cover the rest. Decide with the owner whether an explicit "High contrast" choice without OS forced colours needs its own fixed palette (recommendation: no; system colours resolve to black/white defaults, which pass).
- **DTCG format version**: the spec changed several times before its first stable version; pin the version you follow in the README and keep the generator tolerant of `$type` inheritance.
- **Token names in ADR 0024 vs the design language** may differ (e.g. `text-2` vs `text-secondary`); the ADR wins, and the design language is updated in this PR.
- **The canvases keep literal hex values**; they drift when tokens change. The paper-design plan's §5 rule 3 makes the canvas follow the tokens at its next edit; list changed values in the PR so the canvas owner can update them.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
