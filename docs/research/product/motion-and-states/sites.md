# Sites: interaction states and motion (research)

Scope: `apps/docs` (Starlight 0.42.8 in the Paper look), `apps/site`, `packages/site-kit`, `packages/tokens` as the sites use it. Read on fresh main in `docs-motion`. Starlight and Expressive Code (EC 0.44.3) paths are under `node_modules/.pnpm/…/dist/`.

Headline: **our own code has no transition, animation, `@keyframes`, `:active`, `(hover: hover)` guard, `prefers-reduced-motion` rule or `scroll-behavior` anywhere** (grep over `apps/docs/src`, `apps/docs/runner`, `apps/site/src`, `packages/site-kit/src`). The only motion the sites show comes from Starlight and Expressive Code. The state spec to follow is design language §7.0 (`docs/design/paper/design-language.md:544-559`): hover `hover` fill, pressed `pressed` fill (primary `#123A3F`), disabled `disabled` text with no hover, and `:focus-visible` as a 2 px `focus` ring.

## 1. Cursors

| Element | Cursor today | Evidence | Verdict |
| --- | --- | --- | --- |
| Header nav links, wordmark, "Open the web app" | pointer (anchor default) | `packages/site-kit/src/header.css:22-85` | OK |
| Phone menu `<summary>` | pointer | `header.css:109` | OK |
| Theme select (site-kit) | pointer | `packages/site-kit/src/select.css:48` | OK. Starlight's own Select also sets pointer (`Select.astro:72`) |
| Website code tabs (`<button>`) | pointer, even on the selected tab | `apps/site/src/styles/site.css:240` | Selected tab could be `default` (clicking it does nothing) |
| Starlight tabs (Snippet/Example: TypeScript, Rust, Result, Output) | pointer, because they are `<a role="tab" href="#tab-panel-N">` | Starlight `user-components/Tabs.astro:90-97,133-150` | Pointer is right, but they are links: hovering one shows `#tab-panel-0` in the status bar |
| EC Copy + our "Wrap long lines" | pointer | EC `plugin-frames/dist/index.js` (`cursor: pointer`); `paper.css:341-355` gives Wrap the Copy look | OK |
| Copy as Markdown | pointer, then `progress` while `aria-disabled` | `apps/docs/src/components/CopyMarkdown.astro:92,98-100` | OK |
| Runner: Edit and run, Run, Reset, Download, Wrap lines, Format | pointer, then `progress` when `:disabled` | `Example.astro:644,668-671`; disabled in `Example.astro:269-277`, `runner/editor.ts:244,251,276,296` | Cursor right; nothing else changes (see §2) |
| Code/page divider (≥ 90rem) | `col-resize`, held over the whole page while dragging | `Example.astro:559,579-583` | OK |
| **Starlight search button while disabled** | **pointer** | Starlight `components/Search.astro:176` sets pointer; we only hide its `kbd` while `[disabled]` (`paper.css:307-309`) | **Wrong**: until Pagefind loads (always, in `astro dev`) it looks clickable and does nothing. Needs `button[data-open-modal][disabled] { cursor: default /* or progress */ }` |
| Sidebar groups, mobile ToC `<summary>` | pointer | Starlight `SidebarSublist.astro:94`, `MobileTableOfContents.astro:72` | OK |
| `<details>` in Markdown | pointer | Starlight `style/markdown.css:200` | OK, but no page uses `<details>`, LinkCard or CardGrid today (grep of `src/content`) |
| Example page pictures (`.example-first img`, Result tab) | default | `Example.astro:587-597` | They aren't links. Readers may expect a click to enlarge; the caption link (`Example.astro:116,170`) opens the PDF |
| Current sidebar page | pointer | `paper.css:265-272` | Fine (it reloads the page) |

App note: `@laminapdf/ui` deliberately gives buttons `cursor: default` (`packages/ui/src/styles/button.css:12`, `mode-switcher.css:22`, …), the desktop convention, while both sites use `pointer` on buttons. The plan should say which rule applies where (sites: pointer on every actionable element, which the web convention expects; app: default), so neither side "fixes" the other.

## 2. State coverage

| Element | Hover | Pressed | Focus-visible | Current/selected | Disabled | Touch guard |
| --- | --- | --- | --- | --- | --- | --- |
| Site nav link | text → `text` (`header.css:52`) | none | global ring | 600 + accent underline (`header.css:57-61`) | n/a | none |
| Wordmark | none: `:hover` keeps the same colour (`header.css:29-31`) | none | global | n/a | n/a | — |
| "Open the web app" | `hover` fill (`header.css:82-85`) | none | global | n/a | n/a | none |
| Phone menu summary | `hover` fill, also while `[open]` (`header.css:117-120`) | none | global | open = hover fill | n/a | none: after a tap the fill sticks on iOS |
| Phone menu links | `hover` fill (`header.css:156`) | none | global | accent side rule (`header.css:162-166`) | n/a | none |
| Site-kit select | **none** | none | own ring, offset tight (`select.css:57-60`) | n/a | none | `pointer: coarse` sizing only (`select.css:73-78`) |
| Website `.button-primary` / `.button-secondary` | `accent-strong` / `hover` fill (`site.css:73-76,140-143`) | **none** (§7.0 asks for `#123A3F` / `pressed`) | global | n/a | n/a | none |
| Website code tab | **none** | none | global | text + accent underline (`site.css:244-247`) | n/a | — |
| Website body links | `accent-strong` (`site.css:32-34`) | none | global | n/a | n/a | none; no `:visited` style, which is fine here |
| Docs Starlight tabs | **none** (Starlight has no tab hover, `Tabs.astro:133-150`) | none | Starlight inset offset, our colour (`paper.css:235-237`) | accent underline, 600 | n/a | — |
| Docs hero buttons (Starlight LinkButton) | **none in effect**: `.primary:hover` sets the same colour (`LinkButton.astro:48`); also a 999rem pill radius, not Paper's 4 px | none | Starlight | n/a | n/a | — |
| Copy as Markdown | `gray-6` fill (`CopyMarkdown.astro:94-97`) | none | global | n/a | `progress` cursor only | none: the fill sticks after a tap |
| Runner buttons | **none** (`Example.astro:631-652`) | none | global | Wrap lines `aria-pressed` = accent tint (`Example.astro:648-652`) | **cursor only**: no `disabled` text (§7.0), so Run looks live while running | — |
| Divider | 1 → 3 px accent, instant (`Example.astro:572-577`) | dragging = same | same as hover | n/a | n/a | `touch-action: none` (`:560`) |
| Search button | Starlight border/colour (`Search.astro:193`) | none | Starlight | n/a | **pointer, no look** (see §1) | — |
| Sidebar current page | pinned, accent tint + bar (`paper.css:265-272`) | none | Starlight | yes | n/a | — |
| Pagination | Starlight border colour (`Pagination.astro:65`), Paper size (`paper.css:219-230`) | none | Starlight | n/a | n/a | — |

Cross-cutting:
- **Pressed**: no `:active` rule anywhere; `--paper-pressed` (`tokens.css:232,343`) and `--paper-disabled` (`:233,344`) are unused by both sites.
- **Touch**: no `@media (hover: hover)` guard in our CSS, so every hover fill above sticks after a tap on iOS/Android. EC guards its own Copy button; we don't.
- **Focus**: the docs only recolour Starlight's outline (`paper.css:235-237`, offsets left to Starlight); the website sets ring, offset 2 *and* `border-radius: var(--paper-radius-3)` on every focused element (`site.css:36-40`). That radius changes the shape of focused elements that have no radius of their own (code tabs, inline links), so they round off when focused. Two sites, two ring recipes.
- **Disabled vs aria-disabled**: Copy as Markdown already follows §7.0's "stays focusable" pattern (`CopyMarkdown.astro:5,50-64`); the runner uses `disabled`, which drops focus mid-run (`editor.ts:244`).

## 3. Motion today

- **Our code: none.** No `transition`, `animation`, `@keyframes`, `scroll-behavior`, `prefers-reduced-motion`, View Transitions or `ClientRouter` in either site or site-kit (`astro.config.mjs` of both have only `build: { inlineStylesheets: "never" }`, docs `:86`, site `:60`).
- **Tokens**: four durations only (`packages/tokens/tokens.json:1699-1753` → `tokens.css:70-73`): toast 4000 ms, pill fade 1500 ms, progress cycle 1200 ms, tooltip delay 500 ms. **No state-change duration and no easing token.** The reduced-motion block zeroes only the progress cycle (`tokens.css:638-643`).
- **Third-party motion**:
  - Starlight sidebar caret: `transform 0.2s ease-in-out`, **not** reduced-motion guarded (`SidebarSublist.astro:102-104`).
  - Starlight Markdown `<details>` caret: same, guarded with `no-preference` (`markdown.css:231-234`).
  - EC Copy button and its "Copied!" bubble: 0.2 s opacity/background/transform, `ease-in-out` and `cubic-bezier(0.25,0.46,0.45,0.94)`, **no reduced-motion guard** (0 matches in `plugin-frames/dist/index.js`). Our Wrap button inherits these.
- **Reduced motion**: no site-wide reset like the app's (`packages/ui/src/styles/settings.css:217-229`, the `paper.motion` layer that removes animations, transitions and smooth scroll).
- **Scroll**: no `scroll-behavior` anywhere, so the website's in-page links (`Parts.astro:31-34` → `#packages`, `#m2`, `#m3`; `Opening.astro:29`) and Starlight's contents rail jump instantly. That is correct for long reading. The website header isn't sticky (`Base.astro:62-64`), so `scroll-margin` isn't needed there; Starlight's header is fixed and handles its own offset.
- **Theme switch**: an instant `data-theme` swap (`ThemeSelect.astro:63-67`). The design language allows a transition except under reduced motion (`design-language.md:233`).
- **Page changes**: full reloads with no transition and no prefetch.
- **CSP limits** (ADR 0011 §4, `docs/decisions/0011-…md:105-129`): `style-src 'self'` on both sites (`apps/docs/public/_headers:10`, `apps/site/public/_headers:7`), so no `style=""` attributes and no inline `<style>`; `script-src 'self'` + build-time hashes. The site-kit build check rejects inline styles (`packages/site-kit/src/inline-styles.mjs:1-7`). **Allowed:** rules in style sheets (`transition`, `@keyframes`, `@starting-style`, `@view-transition`, `animation-timeline: scroll()`), CSSOM writes from bundled scripts (precedent: `root.style.setProperty`, `Example.astro:333-336`), the Web Animations API (`el.animate()`) and `document.startViewTransition()`. **Avoid:** Astro's `ClientRouter`, which Starlight doesn't support and which adds a client router for little gain; any library that writes `style` attributes into the markup.

## 4. Where motion would add delight, and where it shouldn't

These are recommendations, not decisions. All of them need a new `--paper-duration-*`/`--paper-ease-*` token pair (e.g. 120 ms state, 180 ms enter, one standard ease) set to 0 under reduced motion, plus a site-wide reduced-motion reset.

- **Hover and pressed (both sites)**: 100–150 ms `background-color, color, border-color, box-shadow` on buttons, tabs and links, gated by `(hover: hover)`. Pressed is instant on the way in and eases out. This one change fixes the abrupt in and out of hover.
- **Tabs** (website code panel, Starlight tabs): ease the underline colour and text colour. A short fade-in of the new pane (opacity only, `@starting-style` on `[role=tabpanel]:not([hidden])`) is fine; never animate height.
- **Docs header on scroll**: a hairline or `shadow-float` that fades in after the first ~8 px, done with a scroll-driven animation (`animation-timeline: scroll()`; CSS only and CSP-safe; browsers without it keep today's hairline). Keep the website header static.
- **Copy feedback**: EC already shows "Copied!". Copy as Markdown writes "Copied" once and never clears it (`CopyMarkdown.astro:52-56`): swap the icon to a check for ~1.5 s with a fade, then restore it and clear the status.
- **Runner Run → Result**: today Run only disables and writes "Running…" (`editor.ts:244-246`), then the picture is hidden and the canvases appear at once, while Download appears and pushes Wrap/Format along the bar (`editor.ts:257-262`). Show a busy Run (the approved indeterminate bar, `--paper-duration-progress-cycle`, static under reduced motion), keep the old picture's space until the pages arrive, fade the pages in (~150 ms), and reserve Download's slot so the bar doesn't jump.
- **Theme switch**: a 150–200 ms cross-fade through `document.startViewTransition()` around `onThemeChange` (`ThemeSelect.astro:63`), skipped under reduced motion.
- **Page transitions**: a cross-document `@view-transition { navigation: auto; }` cross-fade (~150 ms), with the header given a `view-transition-name` so it stays put. This is CSS only (Chromium and Safari; other browsers just navigate) and turned off under reduced motion.
- **Phone menu / divider**: the menu panel fades and moves 4 px in on open (`@starting-style`); the divider eases from 1 to 3 px.
- **Keep still**: long reading pages (guides, reference, the one-page views, Benchmarks tables, How we test figures, the roadmap timeline): no scroll-reveal, parallax, smooth scrolling for long jumps, or counting-up numbers. Search results and the sidebar stay instant. The caret rotations stay but follow reduced motion.

## Top 8 problems, ranked

1. **No pressed state anywhere.** There is no `:active` rule in either site, `--paper-pressed` is unused, and §7.0 specifies one (`design-language.md:548-552`).
2. **Hover with no touch guard.** No `(hover: hover)`, so fills stick after a tap: phone menu summary (`header.css:117-120`), "Open the web app" (`:82`), Copy as Markdown (`CopyMarkdown.astro:94`), website buttons (`site.css:73,140`).
3. **Disabled states are invisible, and one cursor is wrong.** The Starlight search button keeps `cursor: pointer` while disabled (`Search.astro:176`; `paper.css:307` only hides the `kbd`). Runner buttons change only the cursor (`Example.astro:668-671`), with no `disabled` text; `--paper-disabled` is unused.
4. **Missing hover states**: website code tabs (`site.css:233-247`), Starlight tabs (`Tabs.astro:133-150`), the site-kit select (`select.css:37-60`), every runner button (`Example.astro:631-652`), and the docs hero buttons (no visible change, and a pill radius off the Paper look, `LinkButton.astro:33,48`).
5. **No motion tokens or reduced-motion layer for the sites.** There is no state duration or easing token (`tokens.json:1699-1753`), no site-wide reduced-motion reset like the app's (`settings.css:217-229`), and the motion we ship from Starlight's sidebar caret and EC's Copy ignores reduced motion (`SidebarSublist.astro:103`, EC frames).
6. **Every state change is instant**, so hover snaps in and out, including the divider's 1 → 3 px jump (`Example.astro:572-577`). Fix it with short tokenised transitions once items 1, 2 and 5 are done.
7. **The runner's Run → Result is abrupt.** It has no busy look, the picture swaps for the canvases at once, Download's arrival shifts the bar (`editor.ts:244-265`), and `disabled` drops keyboard focus mid-run (§7.0's blocked pattern uses `aria-disabled`).
8. **The focus ring differs between the sites and changes element shape on the website.** `site.css:36-40` adds `border-radius` to every focused element (tabs and links round off); the docs keep Starlight's offsets (`paper.css:232-237`). Copy as Markdown's "Copied" never clears either (`CopyMarkdown.astro:52-56`).
