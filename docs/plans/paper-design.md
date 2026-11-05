# Plan: Paper, lamina's visual design

- **Status:** Draft
- **Date:** 2026-10-29
- **Decision:** the project owner chose the **Paper** direction on 2026-10-29, from five reviewed directions (§2).
- **Review status:** the screens are drawn and documented; per-area design reviews are deferred (owner, 2026-10-29) and tracked in §9.
- **Builds on:** [UI/UX plan](ui-ux.md) (principles, layout, modes, trust surfaces; Round U), [web reader](web-app.md), [desktop reader](desktop-app.md), [full editor](full-editor.md), [hosted service](full-editor.md#e18-collaboration-hosted-service)
- **Design files:** [Paper design language](../design/paper/design-language.md), [screen inventory](../design/paper/screens.md), one document per feature area (§4), the Design canvases, and their source files committed under [`docs/design/paper/`](../design/paper/)

## 1. Goal

Give every screen lamina will ship one visual language, drawn before it is built: the reader on web, desktop and phone, every editor mode, the desktop shell, and the account and product pages. Engineers then build from a reviewed screen and a written description, not from a sentence in a todo, and the UI looks like one product however many rounds and people build it.

This plan answers the UI/UX plan's open question 3 ("when does a visual design pass happen?"): now, before Round C's shell (C4a) and U1a's ADR 0024 fix the tokens.

## 2. The decision: Paper

Five directions were designed against the same brief (web reader, desktop Comment mode with the palette, phone reading and selection, style tile), each reviewed for plan conformance, WCAG 2.2 AA and format, and fixed:

| Direction | Personality | Outcome |
| --- | --- | --- |
| **Paper** | Calm, editorial, book-like: warm off-white chrome, ink text, deep teal accent, Newsreader headings with Source Sans 3 UI | **Chosen** |
| Instrument | Dense, dark-first, keyboard-first pro tool | Not chosen; its shortcut-chip and single-key rules are kept in the design language |
| Native | Looks like it shipped with the OS | Not chosen |
| Signal | Bold, high-contrast, accessibility-first | Not chosen; its prominence of trust surfaces informs Paper's network indicator and save banner |
| Studio | Friendly, soft consumer app | Not chosen |

Paper fits the research's primary personas (people who read long documents: boards, legal, academic) and principle 1, "open into reading": the chrome recedes and the page carries the screen.

## 3. What exists

- [`design-language.md`](../design/paper/design-language.md): tokens (light and dark themes, page appearances), type, spacing, icons with SVG, layout sizes, components with HTML snippets, patterns, copy rules. Values marked **(proposed)** were not in the original boards; the owner signed off those that `@laminapdf/tokens` holds in U6a (2026-11-05), and the markers left wait for a decision.
- [`screens.md`](../design/paper/screens.md): about 110 screens in nine areas, each with platforms and the todos that build it.
- One area document per canvas (§4): for every screen its purpose, entry points, layout, controls with command ids, states, behaviour, accessibility and platform differences.
- The canvases themselves (links in each area document) and their source files in `docs/design/paper/<area>/`. The `.dc.html` files are reference copies: they render in the Design canvas, not standalone, and they are not product code.

## 4. Areas

| Area | Document | Screens | Builds into |
| --- | --- | --- | --- |
| Foundations | [screens.md §F](../design/paper/screens.md#f-foundations-the-chosen-paper-boards) | F01–F05 | U1a–U1c, C4a, U3a |
| Reader A | [reader-a.md](../design/paper/reader-a.md) | R01–R09 | Rounds C3–C6 |
| Reader B | [reader-b.md](../design/paper/reader-b.md) | R10–R18 | C7, U2c, U3a, U4 |
| Desktop shell | [desktop-shell.md](../design/paper/desktop-shell.md) | S01–S10 | Rounds D3–D7 |
| Edit session and organise | [edit-session-organise.md](../design/paper/edit-session-organise.md) | O01–O08 | E1, E2 |
| Pages, navigation and extraction | [pages-navigation-extraction.md](../design/paper/pages-navigation-extraction.md) | O09–O17 | E2, E3 |
| Comment and review | [comment-review.md](../design/paper/comment-review.md) | C01–C09 | E4, E5 |
| Add and edit content | [add-edit-content.md](../design/paper/add-edit-content.md) | A01–A07 | E6, E7 |
| Forms | [forms.md](../design/paper/forms.md) | P01–P07 | E8, E9 |
| Sign, protect and redact | [sign-protect-redact.md](../design/paper/sign-protect-redact.md) | G01–G11 | E10–E12 |
| Convert, compare and produce | [convert-compare-produce.md](../design/paper/convert-compare-produce.md) | X01–X12 | E13–E17 |
| Account, cloud and product | [account-cloud-product.md](../design/paper/account-cloud-product.md) | H01–H11 | H1–H4, E18, U5 |

## 5. How the designs are used

1. **Every UI todo links its screens.** Each todo that builds a screen lists it under **Context** ("Design: …"). Its acceptance includes "matches the linked Paper screens; any deviation is recorded in the area document".
2. **Changing a design.** A PR that changes a screen updates the area document and, when the look changes, the canvas and its committed files in the same PR. Small implementation details (exact spacing found in code) don't need a canvas change; behavioural or visual changes do.
3. **Tokens come from one place.** U6a turns the design language's tokens into `@laminapdf/tokens`; from then on the design language cites that package's file as the source of truth, and the canvases are updated to match it when they drift.
4. **Feature plans own behaviour; design owns look.** Where an area document and a feature plan disagree on behaviour, the feature plan (and its ADRs) wins and the area document is corrected.

## 6. Round U6: Paper in code

The design system is built once, in `@laminapdf/tokens` and `@laminapdf/ui`, before the shell and the editor rounds use it. U6 runs after U1a (ADR 0024 fixes token layers and component rules) and alongside C4.

### U6. Design system

**Deliverable:** tokens package (light, dark, high contrast, page appearances), self-hosted fonts, the icon set, core components and overlays in `@laminapdf/ui`, and a component gallery with visual and accessibility tests.
**Accepted when:** every component in the design language exists with its states; contrast pairs are tested in CI; the gallery renders each component in each theme and screenshot tests guard them; no font or icon is fetched from a third-party host at runtime (the network indicator stays at 0).
**Todos:** [U6a](../todos/done/U6a-paper-tokens.md) (tokens), [U6b](../todos/done/U6b-self-hosted-fonts.md) (fonts), [U6c](../todos/done/U6c-icon-set.md) (icons), [U6d](../todos/done/U6d-core-components.md) (core controls), [U6h](../todos/done/U6h-display-components.md) (display components), [U6e](../todos/done/U6e-overlays-and-sheets.md) (layer stack, dialogs, menus, tooltips, toasts), [U6i](../todos/done/U6i-sheets-positioning-and-palette-shell.md) (bottom sheets, the phone dialog), [U6j](../todos/done/U6j-positioning-and-palette-shell.md) (selection toolbar shell, palette shell), [U6f](../todos/ready/U6f-gallery-and-visual-tests.md) (gallery and visual tests), [U6g](../todos/done/U6g-docs-site-paper.md) (the docs site in Paper, an interim copy until U6a and U6b)

### Changes to existing todos

- [U1a](../todos/done/U1a-adr-0024-app-shell-ux.md): ADR 0024's token layers and visual identity take their values from the Paper design language; the "visual identity" out-of-scope line is resolved.
- [C4a](../todos/ongoing/C4a-ui-chrome-and-platform.md) and every todo that builds UI in Rounds C, D, E, H and U: a "Design" line under Context links its Paper screens (from [screens.md](../design/paper/screens.md)), and acceptance includes matching them.
- [C7a](../todos/ready/C7a-accessibility-and-theming.md): themes are Paper light, Paper dark and high contrast from `@laminapdf/tokens`.

## 7. Risks

| Risk | Mitigation |
| --- | --- |
| Designs drift from code as features are built | §5 rules; U6f's gallery screenshots; area documents updated in the PR that changes behaviour |
| Proposed values (dark theme, warning colour, dialog anatomy) were never reviewed by the owner | Signed off in U6a (2026-11-05) before the tokens were published; later proposals follow the same rule |
| Google Fonts in the canvases vs the privacy promise | U6b self-hosts subsetted fonts (OFL); the app never calls a font CDN |
| Some screens describe features whose ADRs are not written yet (signing, collaboration, JavaScript policy) | The feature's ADR wins; its todo updates the area document |
| A drawn macOS frame is mistaken for the Windows/Linux design | Desktop shell area documents both; S10 shows the Windows frame |

## 8. Open questions

1. ~~Owner sign-off on the proposed values in the design language (dark theme, warning, dialog and form-control styles) — in U6a.~~ Signed off on 2026-11-05 ([U6a](../todos/done/U6a-paper-tokens.md)).
2. A logo and wordmark: the boards use a simple placeholder mark. Who designs the brand mark, and when?
3. Tablet layouts are drawn only for reading and stylus ink; do other modes need tablet-specific designs before E-rounds start?
4. Restricted printing: does the reader ask for confirmation (C6c) or only show the restriction (C4c)? (reader-a R09)
5. Should the tablet reader in v0.1 use the reader top bar (R17)? (reader-b)
6. "History" names both edit history (rail tab) and the revision browser (X03); rename one?
7. Where are app-level requests (update checks) counted, given the network indicator counts per document? (S06)
8. Web `Ctrl F6` region keys wait on a browser collision test.
9. ADR 0021 (JavaScript policy) is pending; the screens use one sentence: "This file contains JavaScript. lamina never runs it."


## 9. To-dos

- [x] Choose a direction (Paper, 2026-10-29)
- [x] Design language, screen inventory and area documents; every screen drawn (205 artboards in 12 canvases)
- [x] Every UI todo links its Paper screens
- [x] Design review: one round per area across all 11 areas, with fixes, and a cross-area consistency pass (2026-10-29)
- [x] Second review with a visual pass on every board at its artboard size with the real fonts, with fixes (2026-10-29): shells kept inside the board, no wrapping counts or chips, drawn check boxes, accent tweak on every board
- [x] Accent colours: Teal, Blue, Plum and Green in light and dark, synced with the account ([design language §2.8](../design/paper/design-language.md#28-accent-colours)); owner decision 2026-10-29
- [ ] Republish the canvases from the committed files (the account's daily publish limit stopped the last publishes)
- [ ] U6a tokens, U6b fonts, U6c icons, U6d core controls, U6h display components, U6e overlays, U6i bottom sheets, U6j palette and selection toolbar shells, U6f gallery and visual tests
- [x] Owner sign-off on proposed values (U6a, 2026-11-05)
