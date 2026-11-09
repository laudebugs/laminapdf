# U6h — `@laminapdf/ui` display components: badges, notices, progress, empty state, status items, thumbnail, outline row, shortcut chip

| Field | Value |
| --- | --- |
| ID | `U6h` |
| Status | `done` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6d](./U6d-core-components.md) |
| ADRs | Follows 0024 (component rules, touch targets; from [U1a](./U1a-adr-0024-app-shell-ux.md)) and 0009 (React 19 chrome, React Aria Components; from [C2a](./C2a-render-adrs.md)) |
| Branch | `feat/u6h-display-components` |
| PR title | `feat(ui): add the paper display components with all states and axe tests` |
| Size | M (≈ 500–700 lines of components and tests) |
| PR | [#239](https://github.com/laudebugs/laminapdf/pull/239) |

## Goal

Finish the Paper component library that [U6d](./U6d-core-components.md) starts. U6d builds the controls; this todo builds the items that show state rather than take input: badges and tags, notices and the toast's look, progress and skeletons, the empty state, status-bar items, the page thumbnail, the outline row and the shortcut chip. They follow U6d's rules, so the reader chrome ([C4a](./C4a-ui-chrome-and-platform.md)), the rail ([C6a](C6a-thumbnails-outline-page-labels.md)), the network indicator ([U4a](../ready/U4a-network-indicator-and-privacy-proof.md)) and the overlays ([U6e](../done/U6e-overlays-and-sheets.md)) compose the same look. This todo was split from U6d when U6d was claimed, because the two together passed the 800-line budget.

## Context

- Design language [§7 Components](../../design/paper/design-language.md#7-components): [§7.0 shared states](../../design/paper/design-language.md#70-shared-states), [§7.6 thumbnail](../../design/paper/design-language.md#76-thumbnail), [§7.7 outline row](../../design/paper/design-language.md#77-outline-row), [§7.8 status bar items](../../design/paper/design-language.md#78-status-bar-items), [§7.10 command palette](../../design/paper/design-language.md#710-command-palette) (its shortcut chip), [§7.20 toast and notice](../../design/paper/design-language.md#720-toast-and-notice), [§7.21 empty state](../../design/paper/design-language.md#721-empty-state), [§7.22 progress](../../design/paper/design-language.md#722-progress), [§7.23 badge and tag](../../design/paper/design-language.md#723-badge-and-tag); the HTML snippets there are the reference markup.
- [U6d](./U6d-core-components.md): `packages/ui`, its CSS rules, the `density` context, the test helpers (`test/states.ts`, `test/axe.ts`) and the literal-colour and import-boundary tests. This todo adds to them; it doesn't set up its own.
- Design language [§8.7 touch rules](../../design/paper/design-language.md#87-touch-rules), [§8.9 accessibility checklist](../../design/paper/design-language.md#89-accessibility-checklist-wcag-22-aa); WCAG 2.2 SC 1.4.11, 2.3.3 (reduced motion), 4.1.2, 4.1.3 (status messages).

## Scope

**In scope**

- Components in `packages/ui`, each a named export with typed props, TSDoc and every state from §7.0:
  - **Badge** / **Tag** (§7.23): neutral, accent, positive, warning, danger; always with a word or an icon, never colour alone.
  - **Notice** (inline strip, §7.20) and the **Toast** visual (§7.20). The toast region, queue and stacking are [U6e](../done/U6e-overlays-and-sheets.md)'s.
  - **Progress**: a determinate bar (`role="progressbar"`, `aria-valuenow`, a label), an indeterminate bar (static under reduced motion), and a **Skeleton** block.
  - **EmptyState** (§7.21).
  - **Status bar items** (§7.8): `StatusBarItem` (a button, minimum height 24), `NetworkIndicator` (presentational: a count, an explanation, and a `warning` state when a request went to an unexpected origin), `RepairedNotice`, `PageLabel` (tabular numerals). The data and panels are [U4a](../ready/U4a-network-indicator-and-privacy-proof.md)'s and [C4b](./C4b-web-app-shell.md)'s.
  - **Thumbnail** (§7.6: rail and phone sizes, the current-page outline, `aria-current="page"`, the number below, and an image slot so the viewer supplies the bitmap) and **OutlineRow** (§7.7: two levels, the current row `aria-current="location"`, and an expand and collapse chevron button with `aria-expanded`).
  - **ShortcutChip** (§7.10's chip, used by buttons' tooltips and the palette): renders platform glyphs (`⌥⌘E`) or words (`Ctrl Shift E`) from a key description passed in.
- Right-to-left layout, touch density, forced colours and focus rings as U6d sets them up.

**Out of scope** (and which todo covers it)

- The toast region, dialogs, sheets, menus and tooltips → [U6e](../done/U6e-overlays-and-sheets.md).
- The data behind the status items, the thumbnails' bitmaps and the outline's tree → [U4a](../ready/U4a-network-indicator-and-privacy-proof.md), [C4b](./C4b-web-app-shell.md), [C6a](C6a-thumbnails-outline-page-labels.md).
- Screenshots of every state → [U6f](../done/U6f-gallery-and-visual-tests.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] For each component: types first, then the implementation, CSS from tokens only, then tests through U6d's helpers.
- [x] Keyboard tests for the interactive ones (status-bar items, the outline row's chevron, the thumbnail as a link or button).
- [x] axe tests per component in every state, light, dark and high-contrast theme, left to right and right to left.
- [x] Touch tests: under touch density every interactive element is at least 44 × 44, and at least 24 × 24 on desktop.
- [x] Add the components to `packages/ui/README.md`'s component map and link each §7 section to its component.

## Acceptance criteria

- [x] Every component in Scope exists with every state the design language lists for it, and U6d's §7 mapping test includes each section in Scope.
- [x] Component tests (Vitest browser mode, Chromium, Firefox, WebKit) cover keyboard operation, `aria-current`, `aria-expanded` and the progress bar's values.
- [x] axe-core reports zero violations for every component, state, theme and direction.
- [x] Indeterminate progress doesn't move under `prefers-reduced-motion`.
- [x] No component CSS contains a literal colour, and no component imports a viewer, a store or the command registry (U6d's tests).
- [x] `cargo xtask check` passes.

## Post-merge checks

None.

## Tests

The components' browser tests in `packages/ui`, through U6d's `test/states.ts` and `test/axe.ts`, in three engines.

## Docs to update

- `packages/ui/README.md`: the components added to its map.
- [Design language §7](../../design/paper/design-language.md#7-components): link each section in Scope to its component, and record any deviation found while building.

## Risks and open questions

- **Thumbnail images**: the viewer supplies an `ImageBitmap` or a canvas; the component takes a render prop or a child slot, so it never imports the viewer.
- **Shortcut glyphs**: the key description's shape comes from [U1b](../done/U1b-command-registry-and-palette.md)'s `Shortcut` type (ADR 0024). Until U1b lands, take a minimal local type that U1b replaces.

## Outcome

- `@laminapdf/ui` gains `Badge` (which also draws tags), `Notice`, `Toast`, `ProgressBar`, `Skeleton`, `EmptyState`, `StatusBarItem`, `NetworkIndicator`, `RepairedNotice`, `PageLabel`, `Thumbnail`, `OutlineRow` and `ShortcutChip`, each with its CSS from tokens only. Seven component tokens and `type.outline-nested` (14 px serif) joined `@laminapdf/tokens` with status `from-files`, and the new hover, pressed and progress-fill colour pairs are declared, so the contrast test checks them.
- Decisions, recorded in the design language: an outline row with children has a chevron button before its link, with one name and `aria-expanded`; the toast has no `role`, since U6e's toast region announces it; the warning network indicator swaps the shield for the Warning sign; the toast swaps the `text` and `surface` colours; skeletons are `hairline`; a pressed status item's label turns `text-3`, because `muted` is under 4.5:1 on `pressed`.
- `ShortcutChip` takes a minimal `ShortcutKeys` (`mac`, `other`), which ADR 0024's `Shortcut` satisfies, and the platform from the app.
- Tests: every component's states go through axe in three themes and both directions and through the target-size checks; `display.browser.test.tsx` covers keyboard operation, `aria-current`, `aria-expanded`, the progress bar's values and the indeterminate bar standing still under reduced motion (a Playwright command in `vitest.browser.config.ts`).
- Left out, because the design language still marks them **(proposed)**: the outline tree, the organise tile, the more status-bar items, the badge sets, the count pill, the error page, the drop zone and the file list row.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
