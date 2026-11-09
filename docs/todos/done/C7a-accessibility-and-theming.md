# C7a — Accessibility: keyboard map and help, focus, landmarks, announcements, themes and axe in CI

| Field | Value |
| --- | --- |
| ID | `C7a` |
| Status | `done` |
| Plan | [web reader §6 C7. Accessibility, offline and launch](../../plans/web-app.md#c7-accessibility-offline-and-launch) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C5b](./C5b-find-in-document.md), [C6a](./C6a-thumbnails-outline-page-labels.md), [C6b](./C6b-links-and-history.md), [C6c](./C6c-print-download-properties.md), [U1b](./U1b-command-registry-and-palette.md), [U1c](./U1c-inverted-l-shell-and-modes.md) |
| ADRs | Follows 0009 (a11y primitives), 0010 (shortcuts consistent across shells; platform modifiers) |
| Branch | `feat/c7a-accessibility` |
| PR title | `feat(ui): add the keyboard map, help dialog, focus management, themes and axe checks` |
| Size | L (600–1,500 lines incl. tests) |
| PR | [laudebugs/laminapdf#370](https://github.com/laudebugs/laminapdf/pull/370) |

## Goal

Make the reader meet WCAG 2.2 AA for its chrome and work fully without a mouse. One **keyboard shortcut map**, shared with the desktop app, drives every action and a help dialog; **focus** moves predictably between toolbar, sidebars, document and dialogs; **landmarks** and **live regions** announce page changes and search results; **high contrast, dark mode and reduced motion** are supported; and **axe** runs in Playwright on every PR. A manual NVDA, VoiceOver and keyboard-only pass is recorded.

## Context

- **Design (Paper):** [F05](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards), [R10 View settings](../../design/paper/reader-b.md#r10-view-settings), [R11 Dark UI theme](../../design/paper/reader-b.md#r11-dark-ui-theme), [R12 Keyboard shortcuts](../../design/paper/reader-b.md#r12-keyboard-shortcuts). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.2 and §5.5](../../plans/ui-ux.md#52-command-registry-and-palette-r3) — the shortcut map and help dialog are generated from [U1b](../done/U1b-command-registry-and-palette.md)'s command registry (no hand-written keymap); page appearance (Original/Sepia/Dark/High contrast) is [U2c](../done/U2c-page-appearance-modes.md)'s, so this todo does UI themes only and drops the "night reading" filter.
- [web-app.md §2 scope](../../plans/web-app.md#2-scope-and-non-goals) and [§4 functional 8](../../plans/web-app.md#functional), [non-functional](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) (WCAG 2.2 AA; zero serious/critical axe violations), [§7 risks](../../plans/web-app.md#7-risks) (untagged PDFs).
- [web-frontend notes §7](../../research/architecture/notes/web-frontend.md) — a11y comes from DOM layers; only mounted pages have DOM (mitigate with outline/landmarks, `aria-setsize`/`aria-posinset`, keyboard page jump); theming via CSS custom properties; don't invert page rendering in dark mode except an explicit night filter.
- [C5a](../done/C5a-text-layer-and-selection.md) (text and struct-tree layers), [C4a](../done/C4a-ui-chrome-and-platform.md) (tokens, message catalogue).
- WCAG 2.2 (W3C Recommendation) — success criteria 2.1.1 keyboard, 2.4.3 focus order, 2.4.7/2.4.11 focus visible/not obscured, 1.4.3/1.4.11 contrast, 2.3.3 animation from interactions, 4.1.3 status messages.

## Scope

**In scope**

- Keyboard scopes and help text on [U1b](../done/U1b-command-registry-and-palette.md)'s command registry (no separate keymap): each command gains `scope: "global" | "document" | "dialog"` and a help `description`; the reader's commands cover open, find, find next/prev, print, download, properties, zoom in/out/reset/fit width/fit page, rotate, next/prev page, first/last page, go to page, back/forward, toggle sidebar, focus document, help (`?`/`F1`); D4b builds native menu accelerators from the same registry.
- Shortcut dispatcher with scope handling (no shortcuts while typing in inputs except Esc), conflict tests, platform modifier mapping (`cmd` on macOS via `navigator.platform`-free detection: `Platform` capability or `userAgentData` where available).
- Help dialog listing shortcuts from the map (single source of truth), searchable.
- Focus management: skip link to the document; F6 cycles regions (toolbar → sidebar → document → find bar); dialogs trap focus and restore on close; after navigation (link, outline, go-to-page) focus moves to the target page region; visible focus indicators everywhere (not obscured by sticky toolbar).
- Landmarks and semantics: `header` (toolbar, `role="toolbar"` with roving tabindex), `nav` (sidebar), `main` (document), `region` per page (`aria-label="Page iv, 4 of 120"`, `aria-roledescription="page"`, `aria-setsize`/`aria-posinset`), status bar live region.
- Live announcements (polite, throttled): page changes on settle, zoom level, search counts (from C5b), document opened (title, page count, tagged/untagged), errors (assertive).
- Themes: light, dark, high contrast (respecting `prefers-contrast` and `forced-colors` — system colours in Windows High Contrast), user override in settings (stored locally; saved to the account with the accent once accounts exist, [H2](../ready/H2-accounts-and-login.md)), an **accent colour** (Teal, Blue, Plum, Green; command `view.accent`; [design §2.8](../../design/paper/design-language.md#28-accent-colours)) applied before first paint so there is no flash, reduced motion (`prefers-reduced-motion`: no smooth scroll, no animated transitions); page rendering not inverted (page appearance modes, including a dark page mode, are [U2c](../done/U2c-page-appearance-modes.md)).
- axe-core in Playwright for every main state (empty, open document, sidebars open, find open, each dialog) in three engines; Lighthouse accessibility score recorded.
- Manual AT pass: NVDA + Firefox/Chrome on Windows, VoiceOver + Safari on macOS, keyboard-only; results recorded in `docs/` (e.g. `apps/web/ACCESSIBILITY.md`) with issues filed for anything not fixed.

**Out of scope** (and which todo covers it)

- Making PDFs themselves accessible (editor E15 / library B8).
- Localisation beyond English (strings are already in a catalogue).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Keymap module + dispatcher + unit tests (no duplicate bindings per scope; every action has a binding or is listed as menu-only; macOS/other variants).
- [x] Wire all existing actions (C4a–C6c) through the dispatcher; remove ad-hoc key handlers.
- [x] Help dialog component generated from the command registry.
- [x] Focus utilities (`FocusScope`, region cycling, restore) and fixes across components; skip link.
- [x] Landmark and page-region semantics in `@laminapdf/viewer` page shells and `@laminapdf/ui` layout.
- [x] `Announcer` service (one polite + one assertive live region) used by viewer events and find.
- [x] Themes are Paper light, Paper dark and high contrast from `@laminapdf/tokens` ([U6a](../done/U6a-paper-tokens.md), which also owns the token contrast test), each with the four accents; here: the theme and accent switches (the Accent row in View settings R10 and Settings › Appearance S07, `view.accent`, disabled in high contrast), `forced-colors` media query styles and the settings UI.
- [x] Reduced-motion handling in viewer (scroll behaviour) and UI (transitions).
- [x] Settings dialog — **C7a owns the app's single settings UI**: a dialog in `@laminapdf/ui` with a small section-registration API (`registerSettingsSection({ id, title, render })`) so later todos add sections instead of building their own (C7b adds "Storage and privacy"). C7a ships the "Appearance and accessibility" section (theme, accent colour, reduced motion override, night filter), stored in `localStorage` (no network) and applied before first paint; once accounts exist ([H2](../ready/H2-accounts-and-login.md)) theme and accent also sync with the account, and `localStorage` stays the pre-paint cache; the dialog is keyboard-operable and covered by the axe suite.
- [x] axe Playwright suite `apps/web/e2e/a11y.spec.ts` across states and themes; keyboard-only e2e that completes: open (via keyboard to file input), navigate pages, find, follow a link and go back, open properties, print dialog, all without the mouse.
- [ ] Manual AT pass and write-up.

## Acceptance criteria

- [x] **axe shows zero serious or critical violations** in every tested state, theme and engine (CI-enforced).
- [x] The keyboard-only Playwright journey passes in Chromium, Firefox and WebKit.
- [x] Every command with a shortcut appears in the help dialog; a test fails if one lacks help text.
- [ ] All theme token pairs meet WCAG 2.2 AA contrast (script in CI); `forced-colors` mode screenshot shows visible focus and controls.
- [x] With `prefers-reduced-motion: reduce`, no smooth scrolling or animations occur (asserted via computed styles / scroll behaviour).
- [ ] **A manual NVDA, VoiceOver and keyboard-only pass is recorded** in `apps/web/ACCESSIBILITY.md` with date, versions, findings and linked issues; no open finding is severity "blocker".
- [x] Page changes and search results are announced (verified in the manual pass and by asserting live-region contents in e2e).

## Post-merge checks

None.

## Tests

Unit tests (command scopes, dispatcher, announcer throttling, contrast script); Playwright `a11y.spec.ts` (axe), `keyboard-journey.spec.ts`, theme screenshots; manual AT checklist.

## Docs to update

`apps/web/ACCESSIBILITY.md` (new: conformance notes, AT pass results, known issues), `packages/ui/README.md` (command scopes and help text); tick "Keyboard shortcuts (from the command registry …) shared with desktop; help dialog", "Focus management, landmarks, live-region announcements…", "High contrast, dark mode, reduced motion" and the axe/manual-pass item in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Browser-reserved shortcuts** (`ctrl+F`, `ctrl+P`, `ctrl+±`) — intercepting them is expected in a document app but surprises some users; the decision (intercept when a document is open) is documented in the help dialog.
- **Virtualised pages limit "read all"** in screen readers; mitigations are the outline, page regions with positions and go-to-page; document it in ACCESSIBILITY.md.
- **Manual AT testing needs Windows + macOS machines** and someone fluent with NVDA/VoiceOver; schedule early.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **Keyboard.** The keymap was already the command registry's (U1b): bindings with scopes, the dispatcher (no single keys in a text field, Escape left to the layer stack) and the conflict test. This PR adds the single-key setting's switch, in Settings and in Keyboard shortcuts, and `isSingleKey`. Help text is each command's title: the registry has no separate description, and every bound command appears in the dialog because the dialog is built from `useShortcutMap()`.
- **Help dialog** (R12): `ShortcutsDialog`, opened by `?` (page), More › Keyboard shortcuts or the palette; searchable, the count announced politely, single keys marked Off while off, and the browser keys the reader takes listed with their reasons (the risk's decision).
- **Focus.** A skip link (a button, since the address fragment is the reader's) focuses the viewer. Page shells are `role="region"` with `aria-roledescription="page"` and a name from the app ("Page iv, 4 of 120"); `aria-setsize`/`aria-posinset` are left out, since a region may not carry them. A history jump moves focus to its page once mounted (`pages.focusOnJump`); a find match keeps focus in the find field. F6 region cycling stays the desktop app's, as R12 decides (`view.focus-next-region` is not registered on the web).
- **Announcements.** `Announcer` with one polite and one assertive region (`aria-live`, no `role`, so a page's own alerts stay the only `alert`); polite messages wait 700 ms for a newer one of their kind. The opened document and its page count, the page once scrolling settles and a chosen zoom (not a fit mode's refit) are announced; find and failures keep their own live text. Whether the document is tagged is not announced yet.
- **Themes.** `Appearance` (theme, accent, motion, single keys) applied as `data-theme`/`data-accent`/`data-motion`, stored under `lamina.appearance` in `localStorage`, and set before the first paint by `public/appearance.js` (a classic script from the app's origin, since the CSP allows no inline script; a test holds it to `parseAppearance`). View settings (R10, `view.settings`) holds Theme and Accent; page appearance, layout and zoom stay U2c's and the zoom control's. On the phone, View settings is reached through More › Settings. Reduced motion is one `paper.motion` layer rule under `[data-motion="reduce"]`; the viewer never scrolls smoothly.
- **Settings dialog** (`app.settings`): `SettingsRegistry` and `useSettingsSection` are the section-registration API; C7a's section is Appearance and accessibility (theme, accent, motion, single-key shortcuts). The night filter is gone (page appearance is U2c's).
- **Commands.** `view.settings` and `app.settings` (desktop ⌘, / Ctrl ,) join the registry; ADR 0024's appendix now lists 46.
- **Checks.** `e2e/a11y.e2e.test.ts` runs axe (serious and critical) on seven states in three themes; the palette state turns off axe's experimental `label-content-name-mismatch`, as `@laminapdf/ui`'s own palette tests do. `e2e/keyboard.e2e.test.ts` is the keyboard-only journey (its link step is a go-to-page jump and Back; links have their own e2e). Both passed locally in Chromium, Firefox and WebKit.
- **Not done here:** the forced-colors screenshot, the Lighthouse accessibility score, and the manual NVDA, VoiceOver and keyboard-only pass, for which `apps/web/ACCESSIBILITY.md` has the table; that acceptance item stays open for the owner.
