# U6d — `@laminapdf/ui` core controls: buttons, toggles, mode switcher, tabs, form controls, swatches, appearance chips

| Field | Value |
| --- | --- |
| ID | `U6d` |
| Status | `done` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6a](./U6a-paper-tokens.md), [U6c](./U6c-icon-set.md) |
| Blocks | [U6e](./U6e-overlays-and-sheets.md), [U6h](./U6h-display-components.md) |
| ADRs | Follows 0024 (component rules, selection surfaces, touch targets; from [U1a](./U1a-adr-0024-app-shell-ux.md)) and 0009 (React 19 chrome, the accessible-primitives library; from [C2a](./C2a-render-adrs.md)); [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) for new dependencies |
| Branch | `feat/u6d-core-components` |
| PR title | `feat(ui): add the paper core components with all states, keyboard and axe tests` |
| Size | L (≈ 700–800 lines of components and tests; split on 2026-11-05 as the Risks below allow: the display items moved to [U6h](../done/U6h-display-components.md)) |
| PR | [#237](https://github.com/laudebugs/laminapdf/pull/237) |

## Goal

Build the Paper component library the shells and editor modes compose: every control in the design language with every state it defines (default, hover, pressed, selected, focus, disabled, error), keyboard behaviour, right-to-left layout and touch sizes, in `@laminapdf/ui`. The components are presentational (props in, events out; no viewer store, no command registry), so [C4a](../ready/C4a-ui-chrome-and-platform.md)'s reader chrome, [U1c](../ready/U1c-inverted-l-shell-and-modes.md)'s shell, [C6a](../ready/C6a-thumbnails-outline-page-labels.md)'s rail and [U4a](../ready/U4a-network-indicator-and-privacy-proof.md)'s network indicator all wire behaviour into the same look instead of styling their own buttons.

## Context

- Design language [§7 Components](../../design/paper/design-language.md#7-components): [§7.0 shared states](../../design/paper/design-language.md#70-shared-states), [§7.2 mode switcher](../../design/paper/design-language.md#72-mode-switcher), [§7.3 icon button](../../design/paper/design-language.md#73-icon-button), [§7.4 text buttons](../../design/paper/design-language.md#74-text-buttons), [§7.5 segmented tabs](../../design/paper/design-language.md#75-segmented-tabs-rail-tabs), [§7.6 thumbnail](../../design/paper/design-language.md#76-thumbnail), [§7.7 outline row](../../design/paper/design-language.md#77-outline-row), [§7.8 status bar items](../../design/paper/design-language.md#78-status-bar-items), [§7.12 colour swatches](../../design/paper/design-language.md#712-colour-swatches), [§7.19 form controls](../../design/paper/design-language.md#719-form-controls-consistent-with-the-page-input-and-note-field), [§7.20 toast and notice](../../design/paper/design-language.md#720-toast-and-notice), [§7.21 empty state](../../design/paper/design-language.md#721-empty-state), [§7.22 progress](../../design/paper/design-language.md#722-progress), [§7.23 badge and tag](../../design/paper/design-language.md#723-badge-and-tag), [§7.24 tool toggle](../../design/paper/design-language.md#724-secondary-toolbar-tool-toggle), [§7.25 page-appearance chips](../../design/paper/design-language.md#725-page-appearance-chips); the HTML snippets there are the reference markup.
- Design language [§8.7 touch rules](../../design/paper/design-language.md#87-touch-rules), [§8.8 keyboard](../../design/paper/design-language.md#88-keyboard-and-shortcuts), [§8.9 accessibility checklist](../../design/paper/design-language.md#89-accessibility-checklist-wcag-22-aa), [§9 writing style](../../design/paper/design-language.md#9-writing-style).
- Design: the Paper foundation boards F01–F05 in [screens.md §F](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards) (F05 is the style tile).
- [U6a](./U6a-paper-tokens.md) (tokens; components read only semantic and component tokens through `var(--paper-…)`), [U6c](./U6c-icon-set.md) (`<Icon>`), [U6b](../ready/U6b-self-hosted-fonts.md) (fonts; not a dependency, the fallback stack works until it lands).
- [C4a](../ready/C4a-ui-chrome-and-platform.md) — plans `packages/ui` with React 19, the React Compiler, the primitives library (recommendation React Aria Components), a message catalogue and a `tokens.css`; see Risks for the order between C4a and this todo.
- [UI/UX plan §5.1](../../plans/ui-ux.md#51-desktop-and-web-layout-r1-r2) and [§5.4](../../plans/ui-ux.md#54-mobile-and-touch-r17) (24 px desktop and 44 px touch targets); [TypeScript guidelines](../../guidelines/typescript.md); [code organization §2](../../guidelines/code-organization.md#2-boundaries-and-dependency-direction).
- WCAG 2.2 SC 1.4.11, 2.4.7, 2.4.11, 2.5.8 and 4.1.2; WAI-ARIA APG patterns: button and toggle button, tabs, radio group, checkbox, switch.

## Scope

**In scope**

- Create `packages/ui` (`@laminapdf/ui`, private) if it doesn't exist: React 19 as a peer dependency via `catalog:`, the primitives library ADR 0009 names (React Aria Components recommended: i18n, RTL and keyboard come with it; licence Apache-2.0), React Compiler in the consumer build, Vitest browser mode with Testing Library and `axe-core`, a nested `biome.json` allowing DOM globals, `"sideEffects": ["*.css"]`, plain CSS per component (CSS modules or `@layer`-scoped files; no CSS-in-JS runtime) using only `var(--paper-…)` tokens.
- Components, each a named export with typed props, TSDoc and every state from §7.0:
  - **Button** variants `primary`, `secondary`, `ghost`, `danger` (ghost) and `danger-filled` (only as a destructive dialog's confirm), sizes `32` / `30` / `26` and `touch` (≥ 44); text-only, icon + text; `loading` state that keeps width.
  - **IconButton**: sizes 26, 28, 32 and 44 (touch, 22 px icon); requires `label` (type-level).
  - **ToggleButton** and **ToolToggle** (§7.24) with `aria-pressed`.
  - **ModeSwitcher**: `role="group"` of `aria-pressed` buttons, active underline, stable height, arrow keys move focus (toolbar-style roving tabindex) or Tab per button (pick one per ADR 0024 and document it).
  - **SegmentedTabs** (§7.5): `tablist`/`tab`/`tabpanel`, automatic or manual activation (decide and document), arrow keys, Home/End.
  - **TextInput**, **TextArea**, **Select** (native `<select>` styled with `appearance: none` and the chevron icon), **Checkbox**, **RadioGroup** (with `<fieldset>`/`<legend>`), **Switch** (`role="switch"`, `aria-checked`): label above, help and error text below wired with `aria-describedby` and `aria-invalid`; 16 px text on coarse pointers.
  - **SwatchGroup** (§7.12): `aria-pressed` circles named by colour, 24 / 28 px, wrapped in 44 px targets on touch; colours from the annotation tokens.
  - **PageAppearanceChips** (§7.25): `role="group"` + `aria-pressed`.
- The display items (badge, tag, notice, the toast visual, progress, skeleton, empty state, status-bar items, thumbnail, outline row, shortcut chip) moved to [U6h](../done/U6h-display-components.md), which builds them on this package.
- Focus ring per §7.0 on every focusable component (`:focus-visible`, never removed; the 1 px and inset variants as props or contexts).
- RTL: every component lays out correctly under `dir="rtl"` (logical properties: `margin-inline-start`, `padding-inline`, `inset-inline-end`; chevrons mirror where direction matters).
- Touch: a `pointer: coarse` media query and a `density` context (`"desktop" | "touch"`) that switch to 44 px targets and 16 px inputs.
- `forced-colors: active`: real borders and outlines remain (tested by screenshots in [U6f](../ready/U6f-gallery-and-visual-tests.md); styles here).
- Strings: components take visible text and labels as props; any built-in text (e.g. "Details") comes from the message catalogue (create the minimal catalogue if [C4a](../ready/C4a-ui-chrome-and-platform.md) hasn't).

**Out of scope** (and which todo covers it)

- Dialogs, bottom sheets, popovers, menus, tooltips, the selection mini toolbar, the command palette shell, the toast region and z-order → [U6e](../done/U6e-overlays-and-sheets.md).
- Reader chrome built from these (toolbar, page indicator, zoom control, drop zone, password dialog, error and repair banners), the `Platform` interface and `ReaderController` → [C4a](../ready/C4a-ui-chrome-and-platform.md).
- Layout regions (top bar, left rail, inspector, status bar, focus mode) → [U1c](../ready/U1c-inverted-l-shell-and-modes.md); phone bottom bar and action bar → [U3a](../ready/U3a-responsive-mobile-and-touch.md); command wiring and shortcut map → [U1b](../ready/U1b-command-registry-and-palette.md).
- Theme switching UI and settings → [C7a](../ready/C7a-accessibility-and-theming.md); the gallery and screenshot tests → [U6f](../ready/U6f-gallery-and-visual-tests.md).
- Inspector panel and save banner (§7.11, §7.13) → [U1c](../ready/U1c-inverted-l-shell-and-modes.md) and [E1e](../ready/E1e-save-and-constraint-banner.md), built from these components.

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`. Load the project's TypeScript guidelines; check whether `packages/ui` exists (C4a may have landed) and extend it rather than re-scaffolding.
- [x] Scaffold or extend `packages/ui` as in Scope; import `@laminapdf/tokens/tokens.css` once in a `styles/base.css` (root font, colour, `button { font: inherit; color: inherit }`).
- [x] Licence-check new dependencies (`react-aria-components` and its `@react-aria/*`/`@internationalized/*` packages, `axe-core` MPL-2.0 as a dev dependency only, Testing Library MIT); `cargo xtask npm-licenses` passes.
- [x] For each component: types first (`index.ts` exports reviewed in the PR), then implementation, CSS from tokens only (a test greps component CSS for hex colours and `px` values that are not tokens, except the 1 px border width), then tests.
- [x] A shared `test/states.ts` helper that renders a component in each of its states (forcing hover/pressed via data attributes for screenshots, real events for behaviour tests) so [U6f](../ready/U6f-gallery-and-visual-tests.md) can reuse it.
- [x] Keyboard tests per component (Tab order, Space/Enter activation, arrows in groups and tabs, Home/End, Escape not swallowed).
- [x] axe tests per component in every state, light and dark theme, LTR and RTL.
- [x] Touch tests: under `density="touch"` every interactive element's bounding box is ≥ 44 × 44; under desktop ≥ 24 × 24.
- [x] `packages/ui/README.md`: component map (design-language section → component → props), the presentational rule (no stores, no commands), how to add a component.

## Acceptance criteria

- [x] Every component in Scope exists with every state the design language lists for it; a test maps each design-language §7 section in Scope to an exported component (a table in the test, so a missing one fails).
- [x] Component tests (Vitest browser mode, Chromium, Firefox, WebKit) cover keyboard operation for each interactive component, `aria-pressed` / `aria-selected` / `aria-checked` / `aria-current` / `aria-expanded` where applicable, and error text wiring (`aria-describedby`, `aria-invalid`).
- [x] axe-core reports zero violations for every component, state, theme (light, dark, high contrast) and direction.
- [x] Target sizes: ≥ 24 × 24 on desktop density and ≥ 44 × 44 on touch density, measured in tests.
- [x] No component CSS contains a literal colour; all colours come from `@laminapdf/tokens` (test).
- [x] The components render correctly with `dir="rtl"` (asserted via computed logical positions in a test; visual check in [U6f](../ready/U6f-gallery-and-visual-tests.md)).
- [x] No component imports a viewer, a store or the command registry (lint test on imports).
- [x] Matches the linked Paper screens (F01–F05); any deviation is recorded in the design language.
- [x] `cargo xtask check` passes.

## Post-merge checks

None.

## Tests

`packages/ui/src/components/**/*.test.tsx` (Vitest browser mode, three engines, Testing Library, `axe-core`): states, keyboard, ARIA, RTL, target sizes; `packages/ui/test/states.ts` and `test/axe.ts` helpers shared with [U6e](../done/U6e-overlays-and-sheets.md) and [U6f](../ready/U6f-gallery-and-visual-tests.md); import-boundary and literal-colour lint tests.

## Docs to update

- `packages/ui/README.md` (new or extended).
- [Design language §7](../../design/paper/design-language.md#7-components): link each section to its component; record any deviation found while building (exact spacing, state details) and drop **(proposed)** where the owner approved it in [U6a](./U6a-paper-tokens.md).
- `CLAUDE.md` layout: `packages/ui` (if this PR creates it).

## Risks and open questions

- **Order with C4a (decided here: U6d before C4a).** C4a creates `@laminapdf/ui` and its chrome; built first, its buttons, dialogs and `tokens.css` would be restyled and partly rewritten once U6d and [U6e](../done/U6e-overlays-and-sheets.md) land. Building the components first avoids that rework at little schedule cost: C4a also waits for [C3b](./C3b-viewer-virtualiser-and-zoom.md)'s viewer, which is the longer path, and U6a–U6e need only U1a. So this todo does **not** depend on C4a; it creates `packages/ui` itself, and **C4a should depend on [U6e](../done/U6e-overlays-and-sheets.md)** (which needs U6d), drop its own scaffold and `tokens.css` tasks, and compose these components (its `PasswordDialog` uses U6e's dialog). Update C4a's **Depends on** in the claim PR of this todo (or in a board-only PR) so the board shows the order.
- **Primitives library**: if ADR 0009 does not name one, decide here (React Aria Components recommended) and amend ADR 0009 in this PR; it is a long-lived dependency.
- **Size**: the list was long, so it was split when this todo was claimed: this todo keeps the controls, and [U6h](../done/U6h-display-components.md) takes the display items.
- **The styles of §7.18–§7.23** were signed off in [U6a](./U6a-paper-tokens.md) (2026-11-05); if a style changes later, only its CSS changes.
- **Mode switcher keyboard model** (roving focus vs Tab stops) must match ADR 0024 and U1c; settle it in this PR and note it in the design language.

## Outcome

- `@laminapdf/ui` holds `Button`, `IconButton`, `ToggleButton`, `ToolToggle`, `ModeSwitcher`, `SegmentedTabs` with `TabPanel`, `TextInput`, `TextArea`, `Select`, `Checkbox`, `RadioGroup` with `Radio`, `Switch`, `SwatchGroup` and `PageAppearanceChips`, on React Aria Components 1.22.1 (Apache-2.0), with `DensityProvider` and `useDensity`. The stylesheet is `@laminapdf/ui/styles.css`: one file per component in `@layer paper.components`, every length and colour from a token. The 23 sizes the components needed became component tokens in `@laminapdf/tokens`, with status `from-files`.
- Decisions, recorded in the design language: each mode button is its own Tab stop; tabs activate automatically, `keyboardActivation="manual"` for a slow panel; the switch is a hidden checkbox input with `role="switch"` inside its label.
- The tests: Node checks for literal colours, `px` values, unknown tokens, the import boundary and the §7 map; browser tests (three engines) for keyboard operation, ARIA state, axe in light, dark and high contrast, left to right and right to left, target sizes at both densities, 16 px touch fields and the mirrored chevron.
- Differences from the plan: the tests drive React directly with `userEvent` from Vitest, without Testing Library; the shared examples are `test/states.tsx`, and axe runs in `test/browser/a11y.browser.test.tsx`. Hover and pressed are not forced through data attributes, so shipped code carries no test hooks; [U6f](../ready/U6f-gallery-and-visual-tests.md) can hover with the browser. No message catalogue: the components have no text of their own.
- Left for later: the design language's "More controls **(proposed)**" in §7.19 (password field, chips input, segmented radio group, toggle group, radio card, anchor grid, stepper) and the phone sizes still marked **(proposed)**, until the owner approves them.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
