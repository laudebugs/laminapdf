# C7a — Accessibility: keyboard map and help, focus, landmarks, announcements, themes and axe in CI

| Field | Value |
| --- | --- |
| ID | `C7a` |
| Status | `ready` |
| Plan | [web reader §6 C7. Accessibility, offline and launch](../../plans/web-app.md#c7-accessibility-offline-and-launch) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C5b](./C5b-find-in-document.md), [C6a](./C6a-thumbnails-outline-page-labels.md), [C6b](./C6b-links-and-history.md), [C6c](./C6c-print-download-properties.md) |
| Blocks | [C7b](./C7b-offline-pwa.md), [C7c](./C7c-performance-gates-and-launch.md), [D4b](./D4b-native-menus.md) |
| ADRs | Follows 0009 (a11y primitives), 0010 (shortcuts consistent across shells; platform modifiers) |
| Branch | `feat/c7a-accessibility` |
| PR title | `feat(ui): add the keyboard map, help dialog, focus management, themes and axe checks` |
| Size | L (600–1,500 lines incl. tests) |
| PR | — |

## Goal

Make the reader meet WCAG 2.2 AA for its chrome and work fully without a mouse. One **keyboard shortcut map**, shared with the desktop app, drives every action and a help dialog; **focus** moves predictably between toolbar, sidebars, document and dialogs; **landmarks** and **live regions** announce page changes and search results; **high contrast, dark mode and reduced motion** are supported; and **axe** runs in Playwright on every PR. A manual NVDA, VoiceOver and keyboard-only pass is recorded.

## Context

- [web-app.md §2 scope](../../plans/web-app.md#2-scope-and-non-goals) and [§4 functional 8](../../plans/web-app.md#functional), [non-functional](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) (WCAG 2.2 AA; zero serious/critical axe violations), [§7 risks](../../plans/web-app.md#7-risks) (untagged PDFs).
- [web-frontend notes §7](../../research/architecture/notes/web-frontend.md) — a11y comes from DOM layers; only mounted pages have DOM (mitigate with outline/landmarks, `aria-setsize`/`aria-posinset`, keyboard page jump); theming via CSS custom properties; don't invert page rendering in dark mode except an explicit night filter.
- [C5a](./C5a-text-layer-and-selection.md) (text and struct-tree layers), [C4a](./C4a-ui-chrome-and-platform.md) (tokens, message catalogue).
- WCAG 2.2 (W3C Recommendation) — success criteria 2.1.1 keyboard, 2.4.3 focus order, 2.4.7/2.4.11 focus visible/not obscured, 1.4.3/1.4.11 contrast, 2.3.3 animation from interactions, 4.1.3 status messages.

## Scope

**In scope**

- `packages/ui/src/keyboard/keymap.ts`: declarative map `{ action, keys: { mac, other }, scope: "global" | "document" | "dialog", description }` for open, find, find next/prev, print, download, properties, zoom in/out/reset/fit width/fit page, rotate, next/prev page, first/last page, go to page, back/forward, toggle sidebar, focus document, help (`?`/`F1`); exported for desktop menus (D4 builds native menu accelerators from it).
- Shortcut dispatcher with scope handling (no shortcuts while typing in inputs except Esc), conflict tests, platform modifier mapping (`cmd` on macOS via `navigator.platform`-free detection: `Platform` capability or `userAgentData` where available).
- Help dialog listing shortcuts from the map (single source of truth), searchable.
- Focus management: skip link to the document; F6 cycles regions (toolbar → sidebar → document → find bar); dialogs trap focus and restore on close; after navigation (link, outline, go-to-page) focus moves to the target page region; visible focus indicators everywhere (not obscured by sticky toolbar).
- Landmarks and semantics: `header` (toolbar, `role="toolbar"` with roving tabindex), `nav` (sidebar), `main` (document), `region` per page (`aria-label="Page iv, 4 of 120"`, `aria-roledescription="page"`, `aria-setsize`/`aria-posinset`), status bar live region.
- Live announcements (polite, throttled): page changes on settle, zoom level, search counts (from C5b), document opened (title, page count, tagged/untagged), errors (assertive).
- Themes: light, dark, high contrast (respecting `prefers-contrast` and `forced-colors` — system colours in Windows High Contrast), user override in settings (stored locally), reduced motion (`prefers-reduced-motion`: no smooth scroll, no animated transitions); page rendering not inverted; optional "night reading" filter toggle.
- axe-core in Playwright for every main state (empty, open document, sidebars open, find open, each dialog) in three engines; Lighthouse accessibility score recorded.
- Manual AT pass: NVDA + Firefox/Chrome on Windows, VoiceOver + Safari on macOS, keyboard-only; results recorded in `docs/` (e.g. `apps/web/ACCESSIBILITY.md`) with issues filed for anything not fixed.

**Out of scope** (and which todo covers it)

- Making PDFs themselves accessible (editor E15 / library B8).
- Localisation beyond English (strings are already in a catalogue).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Keymap module + dispatcher + unit tests (no duplicate bindings per scope; every action has a binding or is listed as menu-only; macOS/other variants).
- [ ] Wire all existing actions (C4a–C6c) through the dispatcher; remove ad-hoc key handlers.
- [ ] Help dialog component generated from the keymap.
- [ ] Focus utilities (`FocusScope`, region cycling, restore) and fixes across components; skip link.
- [ ] Landmark and page-region semantics in `@lamina/viewer` page shells and `@lamina/ui` layout.
- [ ] `Announcer` service (one polite + one assertive live region) used by viewer events and find.
- [ ] Theme tokens for dark and high contrast; `forced-colors` media query styles; contrast check of all token pairs (script asserting ≥ 4.5:1 text and ≥ 3:1 UI components).
- [ ] Reduced-motion handling in viewer (scroll behaviour) and UI (transitions).
- [ ] Settings dialog — **C7a owns the app's single settings UI**: a dialog in `@lamina/ui` with a small section-registration API (`registerSettingsSection({ id, title, render })`) so later todos add sections instead of building their own (C7b adds "Storage and privacy"). C7a ships the "Appearance and accessibility" section (theme, reduced motion override, night filter), stored in `localStorage` (no network); the dialog is keyboard-operable and covered by the axe suite.
- [ ] axe Playwright suite `apps/web/e2e/a11y.spec.ts` across states and themes; keyboard-only e2e that completes: open (via keyboard to file input), navigate pages, find, follow a link and go back, open properties, print dialog, all without the mouse.
- [ ] Manual AT pass and write-up.

## Acceptance criteria

- [ ] **axe shows zero serious or critical violations** in every tested state, theme and engine (CI-enforced).
- [ ] The keyboard-only Playwright journey passes in Chromium, Firefox and WebKit.
- [ ] Every action in the keymap appears in the help dialog; a test fails if an action lacks help text.
- [ ] All theme token pairs meet WCAG 2.2 AA contrast (script in CI); `forced-colors` mode screenshot shows visible focus and controls.
- [ ] With `prefers-reduced-motion: reduce`, no smooth scrolling or animations occur (asserted via computed styles / scroll behaviour).
- [ ] **A manual NVDA, VoiceOver and keyboard-only pass is recorded** in `apps/web/ACCESSIBILITY.md` with date, versions, findings and linked issues; no open finding is severity "blocker".
- [ ] Page changes and search results are announced (verified in the manual pass and by asserting live-region contents in e2e).

## Post-merge checks

None.

## Tests

Unit tests (keymap, dispatcher, announcer throttling, contrast script); Playwright `a11y.spec.ts` (axe), `keyboard-journey.spec.ts`, theme screenshots; manual AT checklist.

## Docs to update

`apps/web/ACCESSIBILITY.md` (new: conformance notes, AT pass results, known issues), `packages/ui/README.md` (keymap API for desktop); tick "Keyboard shortcuts map shared with desktop; help dialog", "Focus management, landmarks, live-region announcements…", "High contrast, dark mode, reduced motion" and the axe/manual-pass item in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Browser-reserved shortcuts** (`ctrl+F`, `ctrl+P`, `ctrl+±`) — intercepting them is expected in a document app but surprises some users; the decision (intercept when a document is open) is documented in the help dialog.
- **Virtualised pages limit "read all"** in screen readers; mitigations are the outline, page regions with positions and go-to-page; document it in ACCESSIBILITY.md.
- **Manual AT testing needs Windows + macOS machines** and someone fluent with NVDA/VoiceOver; schedule early.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
