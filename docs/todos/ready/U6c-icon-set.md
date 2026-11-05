# U6c — `@laminapdf/icons`: the Paper stroke icon set, `<Icon>`, gallery and grid lint

| Field | Value |
| --- | --- |
| ID | `U6c` |
| Status | `ready` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6a](../done/U6a-paper-tokens.md) |
| Blocks | [U6d](./U6d-core-components.md) |
| ADRs | Follows 0024 (component rules; from [U1a](../done/U1a-adr-0024-app-shell-ux.md)), 0009 (React chrome around a framework-free viewer; from [C2a](../done/C2a-render-adrs.md)), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (licence of any dev dependency) |
| Branch | `feat/u6c-icon-set` |
| PR title | `feat(icons): add @laminapdf/icons with the paper stroke icon set and grid lint` |
| Size | M (≈ 300–400 lines of generator, `<Icon>`, lint and tests; the SVG sources and generated module don't count) |
| PR | — |

## Goal

Make the design language's icons a typed, tested package so every screen uses the same glyphs, drawn the same way: 24-unit grid, 1.5 px stroke, `currentColor`, no fills. `@laminapdf/icons` holds one SVG file per icon and a generated, framework-free module of their markup (usable by the React chrome and by the framework-free viewer alike), plus a small typed React `<Icon>` component on a separate subpath with explicit decorative and labelled modes. A lint test rejects any icon that breaks the grid or stroke rules, a gallery page shows them all at every rendered size, and the README lists which icons each screen area needs, so later rounds add icons by rule rather than by taste.

## Context

- Design language [§5 Iconography](../../design/paper/design-language.md#5-iconography): rules (viewBox `0 0 24 24`, `fill="none"`, `stroke="currentColor"`, `stroke-width="1.5"`, butt caps and miter joins, ~3.5–4.5 px margin, rendered at 14/15, 16, 18, 20, 22 px, `aria-hidden` on the SVG, `aria-label` on icon-only buttons as a verb phrase, hit area is the button) and the 22 icons drawn so far (three marked **(proposed)**: warning, chevron down, check with stroke 2 inside checkboxes).
- Design language [§7.3 icon button](../../design/paper/design-language.md#73-icon-button), [§7.8 status bar items](../../design/paper/design-language.md#78-status-bar-items) (the network shield is the one icon stroked in `accent` while idle), [§7.19 form controls](../../design/paper/design-language.md#719-form-controls-consistent-with-the-page-input-and-note-field) (chevron, check), [§9 writing style](../../design/paper/design-language.md#9-writing-style) (labels for icon-only buttons).
- [Screen inventory](../../design/paper/screens.md) and the area documents listed in the [Paper design plan §4](../../plans/paper-design.md#4-areas): the source for the per-area icon list (area documents that are not in the repository yet are added to the list when they land).
- [C4a](./C4a-ui-chrome-and-platform.md) suggested Lucide (ISC) as an icon source; Paper draws its own set instead, so no third-party icon library is added (record that in C4a's PR if it is still open).
- [TypeScript guidelines](../../guidelines/typescript.md#modules-and-packaging); [code organization §9 generated code](../../guidelines/code-organization.md#9-generated-code) and [§11 when to split a package](../../guidelines/code-organization.md#11-when-to-split-a-crate-or-package-and-when-not-to).

## Scope

**In scope**

- **Decision: a separate package `packages/icons` (`@laminapdf/icons`, private)**, not a module of `@laminapdf/ui`, because the framework-free `@laminapdf/viewer` and non-React surfaces (desktop native menus' template images, the docs site) need the same glyphs without React. It has no runtime dependencies.
- `packages/icons/svg/<kebab-name>.svg`: one file per icon, exactly the design language's markup wrapped in the standard `<svg>` element.
- Generator `scripts/generate.ts` → committed `src/generated/icons.ts` (`@generated` header): `ICONS` (`as const` record of name → inner markup and optional `strokeWidth` override), `IconName` literal union, and `iconSvg(name, { size, title? })` returning a complete SVG string; `--check` mode for CI.
- A React subpath `@laminapdf/icons/react` (React 19 as an *optional* peer dependency via `catalog:`, so the root entry stays framework-free): `src/react/icon.tsx` with `<Icon name size />` (decorative: `aria-hidden="true"`, `focusable="false"`) and `<Icon name size label="…" />` (labelled: `role="img"` + `aria-label`, used only where the icon stands alone and is not inside a labelled button); sizes typed as `14 | 15 | 16 | 18 | 20 | 22 | 32` (32 for empty states). `@laminapdf/ui` ([U6d](./U6d-core-components.md)) re-exports it.
- Accessibility helpers: `iconButtonProps(label)` or a lint rule/test that every `IconButton` has an accessible name (the button component itself is [U6d](./U6d-core-components.md)'s; this todo provides the helper and the test utility).
- **Icon lint** (`src/lint.test.ts`, also runnable as `pnpm --filter @laminapdf/icons lint:icons`): parses every SVG and checks viewBox, root attributes, `stroke-width` 1.5 (or the declared override), no `fill` other than `none` (circles drawn as dots such as More's `r="0.9"` are allowed only through an explicit `dots` allowance), no `style`, `class`, `transform`, `id`, `stroke-linecap`/`stroke-linejoin` other than defaults, only `path`, `circle`, `rect`, `line`, `polyline`, `polygon`, `ellipse` elements, and a bounding box inside the 24 grid with ≥ 2 units of margin (warn when outside the 3.5–4.5 unit target, fail below 2).
- **Icon gallery page** (a static page in `packages/icons/gallery/`, or the first page of [U6f](./U6f-gallery-and-visual-tests.md)'s gallery if that lands first): every icon at 14, 16, 18, 20, 22 px in `text-3`, `accent` and dark-theme colours, with its name and the label convention.
- **The icons each area needs**, in `packages/icons/README.md` (derived from the design language, [screens.md](../../design/paper/screens.md) and the area documents present at the time), marking which exist, which this PR draws, and which the area's own todo draws. This PR draws the reader v0.1 set and the icons [U6d](./U6d-core-components.md) and [U6e](./U6e-overlays-and-sheets.md) need (see Tasks).

**Out of scope** (and which todo covers it)

- Icons for editor modes (Comment, Organise, Fill & Sign, Edit, Sign, Redact …) → the todo that builds each screen draws its icons by these rules and adds them to the design language table and this package (the README list says which).
- Icon buttons, toggles and their states → [U6d](./U6d-core-components.md).
- Brand mark and app icon → [Paper design plan §8 open question 2](../../plans/paper-design.md#8-open-questions) (not a stroke icon).
- Desktop native menu template images → [D4b](./D4b-native-menus.md) (may use `iconSvg`).
- Screenshot tests of the icon gallery → [U6f](./U6f-gallery-and-visual-tests.md).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Scaffold `packages/icons` (`package.json` with `exports`, `"sideEffects": false`, `"private": true`, scripts `generate`, `lint:icons`, `typecheck`, `test`; `tsconfig.json`; README).
- [ ] Copy the 22 icons from [§5](../../design/paper/design-language.md#5-iconography) into `svg/` byte-for-byte inside the standard wrapper; name files after the table (`sidebar.svg`, `find.svg`, `zoom-out.svg`, `zoom-in.svg`, `more.svg`, `close.svg`, `back.svg`, `copy.svg`, `highlight.svg`, `comment.svg`, `underline.svg`, `ink.svg`, `shapes.svg`, `share.svg`, `pages.svg`, `reader.svg`, `network-shield.svg`, `repaired.svg`, `signed-file.svg`, `warning.svg`, `chevron-down.svg`, `check.svg`).
- [ ] Draw the icons the reader v0.1 and U6d/U6e need that the design language lacks, on the same grid, and add each to the design language's §5 table with **(proposed)**: chevron right/left/up (outline expand, menus, back/forward), forward, open file (folder), download, print, rotate clockwise, fit width, fit page, info, error (circle with bar), positive (circle check), lock (password), eye and eye-off (show/hide password), settings, help, keyboard, external link, bookmark, attachment, undo, redo, clear (in search fields). The sheet grabber is a drawn bar, not an icon. Show each new icon to the owner (screenshot in the PR) and get it approved before merge.
- [ ] Write the generator and `--check`; wire `--check` and the lint into `test`.
- [ ] Write the lint with a small SVG path parser (M/L/H/V/C/S/Q/T/A/Z, relative and absolute) computing a conservative bounding box from endpoints and control points (arcs: endpoints ± radius). If you pick a library instead (for example `svg-path-bbox`), check its licence and keep it a dev dependency.
- [ ] `<Icon>` in `src/react/icon.tsx` (nested `biome.json` allowing DOM globals for `src/react/` only) with tests: decorative by default; `label` switches to `role="img"`; size sets `width`/`height` only; colour comes from `currentColor`.
- [ ] Gallery page and a test that it lists every `IconName` (no icon missing from the gallery).
- [ ] Per-area icon list in the README: Foundations/Reader (this PR), Desktop shell, Edit session and organise (insert, rotate left/right, extract, delete, duplicate, history), Comment and review (strike, caret, arrow, text box, eraser, pen, stamp, reply, filter), Add and edit content (text, image, link, transform), Forms (text field, checkbox field, date, required), Sign, protect and redact (signature, certificate, timestamp, lock, redact, sanitise), Convert and compare (OCR, compare, batch, layers, measure), Account and cloud (person, cloud, sync, share link, bell); each with its owning todo from [screens.md](../../design/paper/screens.md).

## Acceptance criteria

- [ ] Every icon in the design language's §5 table exists in `@laminapdf/icons` with identical path data (test compares against the Markdown table).
- [ ] The lint passes for every icon, and fails for a fixture SVG with stroke 2, a fill, a `transform` and a shape touching the edge (negative tests in `test/fixtures/`).
- [ ] `IconName` is a literal union; `<Icon name="nope" />` fails to type-check (a `@ts-expect-error` test).
- [ ] `<Icon>` renders `aria-hidden="true"` without `label`, and `role="img"` with an accessible name with `label` (Testing Library); axe reports no violation on the gallery page.
- [ ] Every icon added in this PR is in the design language's §5 table and was approved by the owner (linked in the PR).
- [ ] The root entry `@laminapdf/icons` imports nothing from React (a test resolves its module graph); `@laminapdf/viewer` can use `iconSvg` without React.
- [ ] `cargo xtask check` passes.

## Post-merge checks

None.

## Tests

`packages/icons/src/*.test.ts`: generator `--check`, lint over `svg/` and negative fixtures in `test/fixtures/`, Markdown-table parity, gallery completeness. `packages/icons/src/react/icon.test.tsx` (Vitest browser mode): decorative and labelled modes, axe on the gallery page. Screenshots of the gallery → [U6f](./U6f-gallery-and-visual-tests.md).

## Docs to update

- [Design language §5](../../design/paper/design-language.md#5-iconography): say that `packages/icons/svg/` is the source of truth for icon markup, add the new icons, and replace "add them to this table when a board introduces one" with "add the SVG to `@laminapdf/icons` (the lint enforces the rules) and the row here".
- `packages/icons/README.md` (new, with the per-area list). `CLAUDE.md` layout: `packages/icons`.

## Risks and open questions

- **Separate package vs a module of `@laminapdf/ui`**: chosen separate for the framework-free viewer; if no non-React consumer appears by the end of Round C, folding it into `@laminapdf/ui` is cheap (one re-export). Note the decision in the README.
- **New icons drawn by engineers** may not match the designer's hand: the lint catches grid and stroke, not style. Owner approval of each new icon in the PR covers taste; when a designer redraws one, only the SVG file changes.
- **The 0.9-radius dots in More** are drawn as stroked circles that read as filled dots; the lint needs an explicit allowance rather than accepting small circles everywhere.
- **Area documents** listed in the plan may not be in the repository yet; the per-area list is then derived from screens.md and the design language, and each area's first UI todo completes its row.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
