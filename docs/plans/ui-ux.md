# Plan: UI/UX for lamina's apps (web, desktop, mobile)

- **Status:** Draft
- **Date:** 2026-10-29
- **Builds on:** the [product and UX research report](../research/product/pdf-app-product-and-ux.md) (recommendations R1–R19) and its [five notes](../research/product/README.md); the [architecture plan](architecture.md) (ADRs 0004, 0009, 0010, 0011) and the [web-frontend notes](../research/architecture/notes/web-frontend.md)
- **Applies to:** [web reader](web-app.md) (C rounds), [desktop reader](desktop-app.md) (D rounds), [full editor](full-editor.md) (E rounds); `@laminapdf/ui` and `@laminapdf/viewer` are shared by both shells
- **Roadmap:** [roadmap.md](roadmap.md) (Round U)

## 1. Goal

Make lamina feel like **the fastest, calmest PDF app on every platform**, and make its architecture's guarantees **visible and verifiable**: no upload, no silent writes, edits that keep signatures valid, redaction that proves itself. The research found that incumbents lose users on trust (broken deals, bloat, silent file damage, leaked redactions), not on missing features ([report §1](../research/product/pdf-app-product-and-ux.md#competitors-fail-on-bloat-broken-deals-and-silent-damage-not-on-missing-features)). This plan sets the interaction model that the C, D and E rounds build into, and adds the UI work that no existing todo covers (Round U).

## 2. Scope and non-goals

**In scope**

- One interaction model for web and desktop: docked "inverted-L" chrome, modes, one command registry feeding the palette, menus, toolbars and shortcut help (ADR 0024).
- Reading quality: selection surfaces, column-aware copy, page appearance modes, a reading-order ground-truth corpus.
- The responsive mobile and touch layout of the web reader, and a spike that decides how native mobile apps would be built.
- Trust surfaces: a network-activity indicator, reader mode (reflow) without a cloud service, and the UI rules the editor rounds follow (explicit save, two named signing paths, redaction mark → review → apply).
- Product promises: a published pricing page with fair-pricing rules (lamina will be priced: owner decision 2026-10-29), published speed targets with a benchmark against other readers, and optional (opt-in, content-free) usage telemetry to test the task-split assumption.

**Non-goals**

- Native mobile apps themselves (after the [U3b](../todos/ready/U3b-mobile-shell-spike.md) spike and ADR 0025; [architecture §8](architecture.md#8-open-questions-for-the-team) Q4).
- Editing features (they stay in the [editor plan](full-editor.md); this plan only sets their UI rules in §5.7).
- AI chat over documents. Local semantic search and OCR come later, off by default (§5.8); which tier includes them is a pricing decision (§5.9).
- A brand identity or visual design language beyond tokens and layout (a later design pass; open question 3).

## 3. Users

The research's three personas, with accessibility as a property of all three rather than a fourth persona ([report §2](../research/product/pdf-app-product-and-ux.md#three-personas-and-two-deal-breakers-that-decide-business-adoption)).

| Persona | What makes them stay | What makes them leave |
| --- | --- | --- |
| Academic / researcher | Fast reading on laptop and tablet; standard annotations written into the file (Readwise and Zotero read them); split view; dark page mode; citation and figure previews later | Annotations locked in an app database; broken multi-column copy |
| Business / legal | Fast form fill; clear e-sign vs certificate signing; compression to fit email (Gmail: 25 MB); true redaction | Recoverable redactions (Manafort 2019, DOJ Epstein files 2025); signatures without legal standing; silent writes |
| Casual mobile | Open from the share sheet, reflowed reading, quick search, finger signature, scan later | Sign-up walls, uploads, paywalls on basic tasks |
| Everyone using assistive technology | Struct-tree reading, keyboard-only operation, reflow with font and spacing controls, high contrast | 75% of screen-reader users expect PDFs to be a problem (WebAIM 2019) |

**Task frequency (estimate, not data).** No public dataset splits PDF use by task. Treat "viewing ≈ 70% of time in app" as a prior for the reader; on the web, organise/convert tasks may be 20–40% of first-visit intents ([report §2](../research/product/pdf-app-product-and-ux.md#the-70--20--8--2-task-split-is-unverified-and-probably-wrong-for-the-web)). [U5c](../todos/ready/U5c-opt-in-usage-telemetry.md) tests both with opt-in, content-free counts (telemetry is optional, owner decision 2026-10-29).

## 4. Principles

Recommendations from the research, adopted as the plan's rules ([ADR 0024](../decisions/0024-app-shell-ux.md) records the ones that are hard to reverse).

1. **Open into reading.** The default is Read mode: the page and a slim top bar, as spare as SumatraPDF. Tools appear when the user expresses an intent (a mode, a selection, the palette), never as a ribbon.
2. **Docked, not floating.** Panels dock; nothing floats over the page except a short-lived selection toolbar on desktop. Figma's UI3 and Acrobat's quick-tools widget both retreated from floating panels after user complaints.
3. **Every action has a visible home.** The palette and shortcuts are accelerators; every command also lives in a menu or toolbar (≈ 80% of Office saves came from the toolbar icon, not Ctrl+S).
4. **One command registry.** Palette, native menus, toolbars, context menus and the shortcut help are views of the same list.
5. **The platform owns touch selection.** On phones and tablets lamina never draws over the OS selection menu; it uses a bottom action bar.
6. **Never change the file without asking.** No auto-save, explicit save, incremental by default; the UI says what a save will do to signatures (ADR 0007).
7. **Prove, don't claim.** Privacy is shown by a network indicator and a strict CSP; redaction by a verification report; speed by published benchmarks.
8. **Perceived speed is designed.** Placeholder → stretched low-res → sharp tiles; aim for interactions that answer within 100 ms and INP ≤ 200 ms (p75), published as stretch goals until a gate measures them.
9. **Accessible by construction.** WCAG 2.2 AA: 24 px minimum targets (44 px on touch), a single-pointer alternative to every drag, focus never hidden by sticky chrome.
10. **Priced, never hostile.** lamina is a paid product (owner decision, 2026-10-29), but the price is visible before any work starts: no paywall discovered after filling a form or editing a file, no watermark added to output without warning, and no changed deal for people who already paid (§5.9). The code stays MIT OR Apache-2.0.

## 5. Design

### 5.1 Desktop and web layout (R1, R2)

```
┌────────────────────────────────────────────────────────────────────┐
│ top bar: file · mode switcher · page n/N · zoom · find · ⌘K · ⋯    │
├──────┬─────────────────────────────────────────────────┬───────────┤
│ left │                                                 │ inspector │
│ rail │                 canvas (pages)                  │ (docked;  │
│ thumbs│                                                 │ only when │
│ outline│                                                │ something │
│ bkmks │                                                 │ selected) │
├──────┴─────────────────────────────────────────────────┴───────────┤
│ status: repaired notice · network indicator · page label           │
└────────────────────────────────────────────────────────────────────┘
```

- **Top bar** (slim): file actions, the mode switcher, page indicator, zoom, find, the Cmd/Ctrl+K entry point, overflow.
- **Left rail**: collapsible; thumbnails, outline, bookmarks, attachments later. Remembers its width and state per user (locally).
- **Inspector**: docked on the right, shown only while an annotation, field, page or object is selected; never covers the page.
- **Modes**: Read (default), Comment, Fill & Sign, Organise, Edit. A mode swaps in **one** secondary toolbar under the top bar (the lightweight form of the ribbon's contextual tabs). The reader v0.1 ships Read only; each editor round adds its mode.
- **Focus mode** (shortcut and palette command): hides both rails and the secondary toolbar; selecting something brings the inspector back temporarily (Figma's "Minimize UI").
- The layout is stable: controls don't appear and disappear outside these rules (Microsoft's progressive-disclosure guidance).

Delivered by [C4a](../todos/done/C4a-ui-chrome-and-platform.md) (shell and top bar) and [U1c](../todos/done/U1c-inverted-l-shell-and-modes.md) (rail, inspector, modes, focus mode).

### 5.2 Command registry and palette (R3)

A command is `{ id, title, keywords, group, shortcut?, icon?, enabled(ctx), run(ctx, args), args? }`, where `args` is a typed schema (page range, zoom percentage, file). The registry lives in `@laminapdf/ui` and feeds:

- the **palette** (cmdk, MIT): fuzzy search over titles and keywords, recents first, a shortcut chip on every row, argument parsing inline (`extract 4-8`, `zoom 150`, `go to 12`) and a pushed page for arguments typed afterwards; the APG combobox pattern (focus stays in the input, `aria-activedescendant` on the active row);
- **native menus** on desktop ([D4b](../todos/ready/D4b-native-menus.md)) and the web overflow menu;
- **toolbars and context menus** (buttons reference command ids, so labels, shortcuts and enabled state never drift);
- the **shortcut help dialog** ([C7a](../todos/done/C7a-accessibility-and-theming.md)), generated, never hand-written.

Shortcuts are declared once per command with per-platform variants (`Mod` = Cmd/Ctrl) and checked for collisions with browser and OS shortcuts by a test. Delivered by [U1b](../todos/done/U1b-command-registry-and-palette.md).

### 5.3 Selection and copy (R4, R5)

- `@laminapdf/viewer` stays headless: it **emits** selection events with client rects and the selected text runs; `@laminapdf/ui` owns every menu.
- **Desktop/web with a fine pointer**: a mini toolbar (`role="toolbar"`) above the first selected line, flipped below on collision, shown on `pointerup` (never during a drag), reachable with Shift+F10, dismissed with Escape. Its actions also exist in the context menu, menu bar and palette.
- **Touch**: no floating toolbar. While a selection exists, a contextual **bottom action bar** (Copy, Highlight, Comment, Share, More) replaces the bottom bar; the OS selection menu is left alone (on iOS, scrolling hides it and code can't bring it back).
- **Copy as paragraph** is the default: column-aware, line breaks joined inside paragraphs, hyphenation repaired from lamina's extraction (B9). **Copy exact** keeps the layout's line breaks. Rectangular (column) selection with Alt/Option-drag, as in Preview.
- Quality is measured, not assumed: [U2b](../todos/done/U2b-reading-order-ground-truth.md) builds a reading-order ground-truth corpus and metric (no public benchmark measures reading order).

Delivered by [C5a](../todos/done/C5a-text-layer-and-selection.md) (text layer and selection events) and [U2a](../todos/done/U2a-selection-surfaces-and-copy-modes.md) (menus and copy modes).

### 5.4 Mobile and touch (R17)

Web reader on phones (responsive PWA, from Round C) and the base for any native app later.

- **Top bar** (title, back, overflow) auto-hides on scroll; **bottom bar**: Search, Annotate (later), Pages, Reader, More.
- **Bottom sheets** with medium and large detents for thumbnails, outline, properties and settings; `viewport-fit=cover` and `env(safe-area-inset-*)` for notches and home indicators.
- **Gestures belong to the viewer**: `touch-action: none` on the canvas; the viewer implements pan, pinch and double-tap zoom so browser zoom doesn't fight it; long-press selects text; drawing needs an explicit mode or a pen `pointerType`.
- **Memory caps**: tile bitmaps ≤ 5 MP each on mobile and a total bitmap budget (iOS Safari caps one canvas at ≈ 16.7 MP and all canvases at 224–384 MB).
- **Targets** ≥ 44 px on touch.

Delivered by [U3a](../todos/done/U3a-responsive-mobile-and-touch.md). Native apps: [U3b](../todos/ready/U3b-mobile-shell-spike.md) compares Tauri 2 mobile with UniFFI and native UI (SwiftUI, Compose) on selection-menu conflicts, Apple Pencil latency, share sheet and Files integration, and access to Vision and ML Kit OCR, then proposes ADR 0025.

### 5.5 Page appearance and themes (R8)

UI themes (light, dark, high contrast, from [C7a](../todos/done/C7a-accessibility-and-theming.md)) and the **accent colour** (Teal, Blue, Plum or Green, in light and dark; [design language §2.8](../design/paper/design-language.md#28-accent-colours)) are separate from **page appearance**, which the user picks per document or globally. Theme and accent are cached on every device and applied before first paint; when the user is signed in they are also saved to the account, so every device matches (first-sign-in rule in the design language §2.8).

| Page mode | How | Notes |
| --- | --- | --- |
| Original | No change | Default, even in dark UI |
| Sepia | Recolour paper and text in the renderer | Raster images untouched |
| Dark | Recolour text and paths in the renderer, after drawing, with images and annotations left as drawn (U2c: PDFium's forced-colours path flattens every fill to one colour) | Never a CSS `invert()`: photos would become negatives |
| High contrast | Follow `forced-colors` system colours | As pdf.js does with `Canvas`/`CanvasText` |

Annotation colours never change (they carry meaning). The gutter around pages has its own token. Delivered by [U2c](../todos/done/U2c-page-appearance-modes.md).

### 5.6 Perceived performance (R6, R7)

The pipeline for every page view ([C3b](../todos/done/C3b-viewer-virtualiser-and-zoom.md)):

1. Placeholders at the page's true aspect ratio, so the layout never shifts.
2. A cached low-resolution bitmap stretched to the current zoom.
3. Sharp 256–512 px tiles for the viewport plus a margin, cancelled on scroll or zoom.
4. During a pinch, a CSS transform only; tiles re-render when the gesture ends.

`RenderBackend` takes `(page, scale, clip rect)` and returns a bounded bitmap; no backend allocates a full-page canvas (ADR 0004, [C2a](../todos/done/C2a-render-adrs.md)). PDFium stays read-only, in a growable-heap WASM build ([C1b](../todos/done/C1b-pdfium-wasm-spike.md)). Sidebars may show skeletons; the canvas never shows a blank page.

**Targets.** The owner adopted the research's desktop cold-start and installer-size numbers as release gates (2026-10-29); the other research numbers stay published stretch goals ([U5a](../todos/ready/U5a-pricing-and-promises-page.md)). All are measured against other readers in [U5b](../todos/ready/U5b-reader-benchmark.md).

| Metric | Release gate | Stretch goal (research) |
| --- | --- | --- |
| Web: first page, 10-page 1 MB PDF, warm cache | ≤ 700 ms ([web-app §4](web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward)) | ≤ 500 ms for a 10 MB PDF |
| Desktop: cold start to interactive window | < 300 ms, macOS, Windows and Linux ([desktop-app §4](desktop-app.md#non-functional-targets-and-release-gates); was ≤ 1 s) | — |
| Desktop installer | < 15 MB per platform, PDFium included (was ≤ 40 MB) | — |
| Interaction responsiveness | No main-thread task > 50 ms while scrolling ([web-app §4](web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward)) | INP ≤ 200 ms (p75); every command answers in ≤ 100 ms |
| Crash-free sessions (opt-in reports) | — | ≥ 99.95% (vendor median) |

### 5.7 Trust surfaces (R9–R13)

- **Network indicator** ([U4a](../todos/done/U4a-network-indicator-and-privacy-proof.md)): a status-bar item that counts requests made since the document opened, from the page, the document worker and the service worker (expected: 0 for local files, range requests to the file's own host for `#url=`), with a panel listing each origin; checked against an independent Playwright request log. The production CSP (`connect-src 'self' https:`, C4b) blocks plain HTTP; for HTTPS the indicator is the proof.
- **Saving** (editor, [E1e](../todos/done/E1e-save-and-constraint-banner.md)): explicit save only, incremental by default; the constraint banner says in plain words what saving will do to signatures before it happens.
- **Annotations and form values** (E4, E8): written into the file with appearance streams, readable by Acrobat, pdf.js, Zotero and Readwise; export of highlights as Markdown.
- **Signing** (E10, [E10j](../todos/ready/E10j-signature-ui.md)): two separately named actions, "Add a signature image" and "Sign with a certificate", each with one sentence on what it proves; a validation badge on open.
- **Redaction** (E12): its own tool and mode, mark → review → apply, then the verification report from [E12d](../todos/ready/E12d-scrub-and-rewrite.md); saving is blocked while the report shows any leak.
- **Organiser** ([E2h](../todos/ready/E2h-page-organiser-ui.md)): multi-select grid, insertion bar (no live swapping), keyboard moves, a "Move to page…" dialog as the non-drag alternative (WCAG 2.5.7), one undo step per batch (ADR 0006).

### 5.8 Reader mode (reflow) and local intelligence (R15, R16)

- **Tier 1, tagged files** ([U4b](../todos/done/U4b-reader-mode-from-structure-tree.md)): derive semantic blocks from the structure tree (ISO 32000-2 §14.8; PDF Association "Deriving HTML from PDF"); about half of web PDFs are tagged since mid-2025. Tables, figures and formulas show as cropped page images with their `/Alt` text; "back to page" returns to the matching position. Font size, line spacing, width and page mode are adjustable.
- **Tier 2, untagged files** ([U4c](../todos/done/U4c-approximate-reader-mode.md)): heuristic layout analysis in pure Rust from lamina's extraction, labelled **"Approximate"**.
- The core emits blocks (bytes-in/bytes-out); `@laminapdf/ui` renders them. No cloud processing and no page cap (Liquid Mode needs Adobe's cloud and skips files over 200 pages).
- **OCR and semantic search** come with E13 and later: OCR behind one trait (platform OCR on native, ocrs or PP-OCR in WASM), model weights licence-audited separately from code (DocLayout-YOLO is AGPL; Marker's weights are OpenRAIL-M with commercial limits).

### 5.9 Product promises (R18, R19)

- **Pricing and promises** ([U5a](../todos/ready/U5a-pricing-and-promises-page.md)): lamina is priced (owner decision, 2026-10-29); the owner sets the model and prices. The page publishes them with the fair-pricing rules from principle 10 (recommended by the research: the grievances were surprise paywalls, changed deals and watermarks, not prices as such): what each tier includes, shown before work starts; existing purchases honoured when the model changes; no ads; no document data used for anything but the user's own work. The research's free-core model (Obsidian) is recorded as the alternative that was not chosen.
- **Benchmark** ([U5b](../todos/ready/U5b-reader-benchmark.md)): reproducible cold start, first page, scroll and memory numbers for lamina against pdf.js, the Chrome viewer, Preview, SumatraPDF and Acrobat Reader on the same reference files.
- **Telemetry** ([U5c](../todos/ready/U5c-opt-in-usage-telemetry.md)): optional (owner decision, 2026-10-29): opt-in, off by default, aggregate command counts only (never document content, names or URLs), to test the task-split assumption and guide which modes ship first. None in v0.1; ADR 0011 records the payload and retention rules.

## 6. Rounds

Round U runs alongside Rounds C, D and E; each todo's "Depends on" fixes where it lands. U1 and the v0.1 parts of U2–U4 gate the web reader launch ([C7c](../todos/done/C7c-performance-gates-and-launch.md)); reflow, the mobile spike and the product-promise todos follow.

Each round is split into todos (one pull request each) on the [todo board](../todos/README.md); they move `ready/` → `ongoing/` → `done/`. When a todo moves, the PR that moves it updates every link to it, including these.

### U1. Shell foundations

**Deliverable:** ADR 0024 (layout, modes, command registry, selection surfaces, page appearance, mobile layout, tokens); the command registry and palette; the left rail, inspector, mode switcher and focus mode in `@laminapdf/ui`.
**Accepted when:** every reader command is in the registry and reachable from the palette, a menu and its shortcut; the help dialog and desktop menus are generated from it; axe reports no serious violations on the shell.
**Todos:** [U1a](../todos/done/U1a-adr-0024-app-shell-ux.md), [U1b](../todos/done/U1b-command-registry-and-palette.md), [U1c](../todos/done/U1c-inverted-l-shell-and-modes.md)

### U2. Reading quality

**Deliverable:** selection toolbar and bottom action bar, copy as paragraph / copy exact / column selection; a reading-order ground-truth corpus with a metric in CI; page appearance modes in both render backends.
**Accepted when:** copy-as-paragraph meets the U2b reading-order threshold on the corpus; Sepia and Dark modes leave opaque raster images pixel-identical to Original, and never recolour any image's samples.
**Todos:** [U2a](../todos/done/U2a-selection-surfaces-and-copy-modes.md), [U2b](../todos/done/U2b-reading-order-ground-truth.md), [U2c](../todos/done/U2c-page-appearance-modes.md)

### U3. Mobile and touch

**Deliverable:** the responsive phone and tablet layout of the web reader; the `mobile-shell` spike with a recommendation for ADR 0025.
**Accepted when:** Playwright mobile profiles (iPhone, Pixel, iPad) pass the reader flows with touch; a 1,000-page file scrolls on a real iPhone without a canvas-memory crash; the spike README answers the four criteria with measurements.
**Todos:** [U3a](../todos/done/U3a-responsive-mobile-and-touch.md), [U3b](../todos/ready/U3b-mobile-shell-spike.md)

### U4. Trust and reflow

**Deliverable:** the network indicator and privacy proof; reader mode from the structure tree, then the approximate reader mode for untagged files.
**Accepted when:** the indicator shows 0 requests for local files in a Playwright test that also counts requests independently; reader mode round-trips position and passes axe; the approximate mode is labelled and scored against U2b's corpus.
**Todos:** [U4a](../todos/done/U4a-network-indicator-and-privacy-proof.md), [U4b](../todos/done/U4b-reader-mode-from-structure-tree.md), [U4c](../todos/done/U4c-approximate-reader-mode.md)

### U5. Product promises

**Deliverable:** the pricing, promises and targets pages on the website (`apps/site`, [website plan §4](website.md#4-the-website)); the reader benchmark; optional, opt-in usage telemetry.
**Accepted when:** the owner has approved the pricing page; the benchmark is reproducible from a script with published raw numbers; telemetry is off by default and its payload is documented and tested to carry no document data.
**Todos:** [U5a](../todos/ready/U5a-pricing-and-promises-page.md), [U5b](../todos/ready/U5b-reader-benchmark.md), [U5c](../todos/ready/U5c-opt-in-usage-telemetry.md)

### U6. Paper design system

Planned in the [Paper design plan §6](paper-design.md#6-round-u6-paper-in-code): tokens, self-hosted fonts, icons, core components, overlays and a gallery with visual tests.
**Todos:** [U6a](../todos/done/U6a-paper-tokens.md), [U6b](../todos/done/U6b-self-hosted-fonts.md), [U6c](../todos/done/U6c-icon-set.md), [U6d](../todos/done/U6d-core-components.md), [U6h](../todos/done/U6h-display-components.md), [U6e](../todos/done/U6e-overlays-and-sheets.md), [U6i](../todos/done/U6i-sheets-positioning-and-palette-shell.md), [U6j](../todos/done/U6j-positioning-and-palette-shell.md), [U6f](../todos/done/U6f-gallery-and-visual-tests.md)

### Changes to existing todos

The research and the owner's decisions on it change todos in other plans (each notes this plan or the decision in its text):

| Todo | Change |
| --- | --- |
| [C1b](../todos/done/C1b-pdfium-wasm-spike.md) | Choose a growable-heap PDFium WASM build; one PDFium instance per worker; record upstream licence notices |
| [C2a](../todos/done/C2a-render-adrs.md) | ADR 0004's `RenderBackend` renders tiles `(page, scale, clip)` into bounded bitmaps; PDFium never writes documents |
| [C3b](../todos/done/C3b-viewer-virtualiser-and-zoom.md) | The four-step render pipeline in §5.6 and the mobile bitmap caps |
| [C4a](../todos/done/C4a-ui-chrome-and-platform.md) | Depends on U1a (ADR 0024 fixes the layout first); `ReaderShell` exposes the rail, inspector and secondary-toolbar slots that U1c fills; buttons reference command ids once U1b lands |
| [C5a](../todos/done/C5a-text-layer-and-selection.md) | The viewer emits selection events with rects and runs for U2a |
| [C6a](../todos/done/C6a-thumbnails-outline-page-labels.md) | Depends on U1c; thumbnails and outline fill the left rail's tabs instead of a sidebar of their own |
| [C7a](../todos/done/C7a-accessibility-and-theming.md) | Depends on U1b and U1c; the keymap and help dialog come from the command registry; the "night reading" filter is replaced by U2c's page modes |
| [C2a](../todos/done/C2a-render-adrs.md) (ADR 0011) | Telemetry optional, opt-in and off by default (owner decision 2026-10-29) |
| [C7c](../todos/done/C7c-performance-gates-and-launch.md) | Also depends on U2a, U2c, U3a and U4a; web gates unchanged (stretch goals are U5a's); launch copy states v0.1's pricing status |
| [D2d](../todos/done/D2d-tile-transport-and-first-document.md), [D2e](../todos/done/D2e-desktop-e2e-webdriverio.md), [D6b](../todos/ready/D6b-release-workflow-and-linux-packages.md), [D7c](../todos/ready/D7c-desktop-beta-release.md) | Desktop gates (owner decision 2026-10-29): cold start < 300 ms measured in D2d and asserted by D2e on the reference runner, installer < 15 MB in D6b's budget step; a miss blocks D7c |
| [D7a](../todos/ready/D7a-crash-reporting.md), [H1](../todos/ready/H1-auth-spike.md), [H2](../todos/ready/H2-accounts-and-login.md) | Usage analytics via U5c only; pricing questions decided once, in U5a |
| [D4b](../todos/ready/D4b-native-menus.md) | Depends on U1b; native menus are generated from the command registry |
| [E2h](../todos/ready/E2h-page-organiser-ui.md) | Insertion bar, "Move to page…" dialog, one undo step per batch |
| [E10j](../todos/ready/E10j-signature-ui.md) | Two named signing actions with plain-language explanations |
| [E12d](../todos/ready/E12d-scrub-and-rewrite.md) | Saving is blocked while the verification report shows a leak |

## 7. Risks

| Risk | Mitigation |
| --- | --- |
| Much of the evidence comes from search snippets (adobe.com, w3.org, webaim.org, pdfa.org and others were unreachable) | [ADR 0024](../decisions/0024-app-shell-ux.md) re-checked the WCAG 2.2, APG, Apple HIG and Material 3 sources it relies on against the primary pages (2026-11-04); they agree with this plan. The PDF Association sources belong to reader mode and are U4b's to re-check |
| A command registry over-engineered before there are many commands | Start with the reader's ~30 commands; the schema grows only when a mode needs it |
| Floating selection toolbar annoys users or blocks text | Shown only after `pointerup`, flips on collision, disable in settings; never the only path |
| Reflow quality on badly tagged files (fewer than 1% claim PDF/UA) | Fall back to tier 2 when the tag tree fails sanity checks; label the result; keep "back to page" one tap away |
| Gesture handling in the viewer breaks browser accessibility zoom | Keep browser zoom working for the chrome; test with iOS and Android zoom and screen readers |
| Page-mode recolouring differs between PDFium and hayro | Render-diff the modes in the nightly lane; Original stays the default |
| Telemetry erodes the privacy promise | Off by default, aggregate counts only, documented payload, a test that it carries no document data, visible in the network indicator |
| Pricing reads as the paywalls users complained about | Price and tier shown before work starts; no feature locked after the user has invested work; honour existing purchases (principle 10) |
| < 300 ms cold start and < 15 MB installer are hard with a sandboxed engine process and PDFium (macOS universal builds double PDFium) | Measure from D2a/D2d onward; per-arch macOS builds if universal breaks 15 MB; show the window before the engine process is ready; Linux (WebKitGTK) is the likeliest miss, so measure it first |

## 8. Open questions

1. ~~Does the owner adopt the free-core pledge?~~ **Answered 2026-10-29:** no, lamina will be priced. Still open: the pricing model (one-time, subscription, or both), what each tier includes, and how licences are checked. On-device features must keep working without an account and offline (architecture principle 6, the hosted-service decision, ADR 0011), so local features need offline licence keys or store purchases; account-based checks fit only hosted services, unless the owner changes that rule. U5a needs these.
2. Tablet stylus annotation (Apple Pencil, S Pen) in the web reader, or only in native apps after ADR 0025?
3. ~~When does a visual design pass (typography, iconography, brand) happen, and who does it?~~ **Answered 2026-10-29:** the owner chose the **Paper** direction; every screen is designed and documented in the [Paper design plan](paper-design.md), and Round U6 builds it into `@laminapdf/tokens` and `@laminapdf/ui`. Still open: a brand mark.
4. Should citation previews (Semantic Reader style) and Zotero integration get their own plan for the academic persona?
5. ~~Telemetry: none at all, or U5c's opt-in counts?~~ **Answered 2026-10-29:** telemetry is optional (opt-in, off by default); U5c goes ahead after v0.1.

## 9. To-dos

Each round's todos (one per PR, with dependencies) are linked from [§6](#6-rounds) and on the [todo board](../todos/README.md); this checklist is the summary.

Ticked by the PR that completes them.

### U1. Shell foundations
- [x] Write ADR 0024 (layout, modes, command registry, selection surfaces, page appearance, mobile layout, tokens): [ADR 0024](../decisions/0024-app-shell-ux.md), with the reader v0.1 command set as its appendix
- [x] Command registry in `@laminapdf/ui` with typed arguments, shortcuts and collision test ([U1b](../todos/done/U1b-command-registry-and-palette.md))
- [x] Command palette with argument parsing, recents and shortcut chips ([U1b](../todos/done/U1b-command-registry-and-palette.md): its own ranking rather than cmdk's, so no dependency)
- [x] Left rail, docked inspector, mode switcher, secondary toolbar slot and focus mode ([U1c](../todos/done/U1c-inverted-l-shell-and-modes.md))

### U2. Reading quality
- [x] Desktop selection toolbar and touch bottom action bar ([U2a](../todos/done/U2a-selection-surfaces-and-copy-modes.md))
- [x] Copy as paragraph, copy exact and rectangular selection ([U2a](../todos/done/U2a-selection-surfaces-and-copy-modes.md))
- [x] Reading-order ground-truth corpus and metric in CI ([U2b](../todos/done/U2b-reading-order-ground-truth.md): 22 generated files, first mean 0.787, ratchet in `fixtures/reading-order/threshold.json`)
- [x] Page appearance modes (Original, Sepia, Dark, High contrast) in both render backends

### U3. Mobile and touch
- [x] Responsive phone and tablet layout: bottom bar, sheets with detents, safe areas ([U3a](../todos/done/U3a-responsive-mobile-and-touch.md))
- [x] Viewer-owned gestures and mobile bitmap budgets ([U3a](../todos/done/U3a-responsive-mobile-and-touch.md))
- [ ] `mobile-shell` spike and a recommendation for ADR 0025

### U4. Trust and reflow
- [x] Network indicator and CSP-backed privacy test ([U4a](../todos/done/U4a-network-indicator-and-privacy-proof.md))
- [x] Reader mode from the structure tree ([U4b](../todos/done/U4b-reader-mode-from-structure-tree.md))
- [x] Approximate reader mode for untagged files ([U4c](../todos/done/U4c-approximate-reader-mode.md))

### U5. Product promises
- [ ] Pricing, promises and targets pages (after the owner sets the pricing model)
- [ ] Reader benchmark against other readers
- [ ] Optional (opt-in), content-free usage telemetry

### U6. Paper design system
- [x] Tokens, fonts, icons, core components, overlays, gallery and visual tests ([Paper design plan](paper-design.md#9-to-dos))
