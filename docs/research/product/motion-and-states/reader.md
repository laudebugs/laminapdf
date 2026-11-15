# Reader interaction states and motion — research

Worktree `docs-motion` at main, 2026-11-14. Paths are relative to the repo root. "§" = `docs/design/paper/design-language.md`.

## 1. Cursors

**The rule today: Paper sets `cursor: default` on purpose, on every control.** Nothing in `packages/ui` sets `cursor: pointer` on a control. The reset (`packages/ui/src/styles/base.css:13-17`) only sets `font`, `color` and `margin` on Paper buttons. It sets no cursor. React Aria adds none either, so a `<button>` would show the UA arrow anyway. The explicit `default` is still a choice: it holds even where a UA would give a pointer (`<a>` styled as Paper rows).

| Element | Cursor today | Where |
| --- | --- | --- |
| Button, IconButton (all variants) | `default` | `button.css:12` |
| Button `[data-pending]` | `progress` | `button.css:118-120` |
| IconButton pending | not styled: no `[data-pending]` rule for `.paper-icon-button` | `button.css:102-106` |
| ToggleButton, ToolToggle | `default` | `toggle.css:19` |
| ModeSwitcher items | `default` | `mode-switcher.css:22` |
| SegmentedTabs / Tab | `default` | `tabs.css:21` |
| Checkbox, Radio, Switch (whole label row) | `default` | `choice.css:11` |
| SwatchGroup swatches | `default` | `swatches.css:31` |
| Accent swatches (Settings) | `default` | `settings.css:61` |
| Page-appearance chips | `default` | `appearance-chips.css:21` |
| Select (native `<select>` in `field.tsx:105-121`) | UA default (arrow) | no rule |
| Menu items | `default` | `popover.css:38` |
| Palette rows | `default` | `palette.css:104` |
| OutlineRow toggle and link (an `<a>`) | `default`, overriding the link's UA pointer | `outline-row.css:21`, `:46` |
| Outline drag grip (bookmark editor) | no rule: arrow, not `grab` | `outline-row.css:171-188` |
| Paper Thumbnail | `default` | `thumbnail.css:15` |
| Reader's real thumbnails (viewer strip, `div[role=option]`) | no rule: arrow | `apps/web/src/app.css:78-113` |
| Document tabs | `default` | `document-tabs.css:65` |
| Status-bar items | `default` | `status-bar.css:18` |
| Toast action | `default` | `notice.css:81` |
| History rows | `default` | `history.css:62` |
| Recent-file rows | **`pointer`** (`missing` rows: `default`) | `recent.css:43`, `:78` |
| Threads toggle and part buttons | **`pointer`** | `threads.css:54`, `:76` |
| Phone bottom-bar items | **`pointer`** | `phone.css:66` |
| Shortcuts `<summary>` | `default` | `settings.css:305` |
| Rail resize handle (`role=separator`) | `col-resize` (while hovered; captured during the drag) | `shell.css:22` |
| Bottom-sheet grabber | `grab` / `grabbing` while dragging | `sheet.css:70`, `:74` |
| Link-layer anchors | inline `pointer`, or `default` when the link goes nowhere | `packages/viewer/src/layers/link-layer.ts:534` |
| Text-layer spans | inline `text` on every span | `packages/viewer/src/layers/text-layer.ts:275` |
| Canvas, select tool | `""` (inherits; arrow off text, I-beam over spans) | `packages/ui/src/tools/tools.ts:67` |
| Canvas, textSelect tool | `text` | `tools.ts:70` |
| Canvas, hand tool | `grab`, **never `grabbing`** while dragging | `tools.ts:77`, `:154-158` |
| Canvas, link tool | `crosshair` | `apps/web/src/links.tsx:468` |
| Disabled controls | no rule anywhere: arrow, same as enabled | — |

Problems:

- **Mixed convention.** Three surfaces use `pointer` (recent, threads, phone bar) and everything else uses `default`. The Paper design files draw `cursor: pointer` on buttons (`docs/design/paper/foundations/project/Main.dc.html:22`, `:76`; `website-home/project/H12-home.dc.html:29`). §7.0 (`design-language.md:545-553`) never names a cursor. The owner's complaint ("buttons don't have the right cursor") is about this rule, and the plan needs a decision on it, written into §7.0.
- **The hand tool shows the wrong cursor.** `attachTools` sets `grab` on the viewer root (`tools.ts:157`), but every text-layer span carries an inline `cursor: text` (`text-layer.ts:275`). With the hand tool on, the cursor over text is still the I-beam. Only `[data-tool=link] .lamina-link` gets an override (`link-editor.css:85-88`), and it changes `pointer-events`, not the cursor. Nothing sets `grabbing` during the drag (`tools.ts:159-170`).
- **The link tool** sets `crosshair` on the root, but the spans' `text` wins over text there too.
- **Drag affordances.** The outline grip has no `grab` cursor. Document tabs drag with `default` (`document-tabs.css:49-51`).
- **Disabled** controls get no `not-allowed` or other cursor. §7.0 does not ask for one. If the plan adopts `pointer`, disabled controls must fall back to `default`.
- **OutlineRow links** are real `<a>` elements, but the cursor is forced to `default` (`outline-row.css:46`).

## 2. State coverage

React Aria's `data-hovered` ignores touch emulation, so the rules below that use it are safe on touch. Plain `:hover` is not, and sticks after a tap on iOS and Android.

| Component (file) | hover | pressed | focus-visible | selected | disabled | pending | invalid | notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Button secondary/primary (`button.css:37-66`) | ✓ | ✓ | ✓ (base.css:23) | — | ✓ | ✓ | — | — |
| Button ghost/danger (`:76-92`) | ✓ | ✓ | ✓ | — | ✓ | ✓ but a hovered pending ghost keeps the hover fill: `:102` (0,2,0) loses to `:76` (0,3,0) | — | — |
| Button `danger-filled` (`:94-99`) | **✗** | **✗** | ✓ | — | ✓ | ✓ | — | no hover or press feedback at all |
| IconButton (`:76-92`) | ✓ | ✓ | ✓ | `aria-expanded` look (`:82-87`) | ✓ | **✗** | — | — |
| ToggleButton / ToolToggle (`toggle.css`) | ✓ | **✗** | ✓ | ✓ | ✓ | — | — | a selected toggle shows no hover: `[data-selected]` (`:27`) comes after `[data-hovered]` (`:22`) at equal specificity |
| ModeSwitcher (`mode-switcher.css`) | ✓ (text colour) | **✗** | ✓ | ✓ | **✗** | — | — | selected goes to weight 600 (`:29-33`), so the label widens on select (layout shift) |
| Tabs (`tabs.css`) | ✓ | **✗** | ✓ | ✓ | ✓ | — | — | the same 600 width jump (`:28-32`) |
| Checkbox/Radio/Switch (`choice.css`) | **✗** | **✗** | ✓ (`:124-130`) | ✓ | ✓ (row text and box only) | — | **✗** | the switch knob jumps, with no transition (`:112-119`) |
| SwatchGroup (`swatches.css`) | **✗** | **✗** | ✓ | ✓ | **✗** | — | — | — |
| Accent swatch (`settings.css:55-88`) | **✗** | **✗** | ✓ | ✓ | ✓ | — | — | — |
| Appearance chips | ✓ | **✗** | ✓ | ✓ | — | — | — | — |
| Text input / Select (`field.css`) | **✗** | — | ✓ | — | ✓ | — | ✓ | Select chevron has no hover |
| Menu item (`popover.css:29-66`) | via `data-focused` (RAC focuses on hover) | **✗** | the same tint as hover | — | ✓ + reason | — | — | destructive ✓ |
| Palette row (`palette.css:129-151`) | via `data-focused` | **✗** | ✓ | — | ✓ | — | — | — |
| OutlineRow (`outline-row.css`) | ✓ | **✗** | ✓ | ✓ current, selected, drop target | — | — | rename field ✓ | — |
| Paper Thumbnail (`thumbnail.css:58-75`) | ✓ | ✓ | ✓ | ✓ current | — | — | — | not used by the reader |
| Reader thumbnails (`apps/web/src/app.css:83-113`) | **✗** | **✗** | ✓ | ✓ | — | — | — | the rail's Pages tab, the most used list |
| Find result (`find-results.css`) | ✓ | **✗** | ✓ | ✓ `aria-current` | — | — | — | — |
| Recent row (`recent.css`) | ✓ | **✗** | ✓ | — | missing ✓ | — | — | — |
| History row (`history.css:48-96`) | plain `:hover` (`:65`) | **✗** | the base ring only | ✓ current/undone | — | — | — | sticky hover on touch |
| Document tab (`document-tabs.css:40`, `:99-106`) | plain `:hover` | **✗** | ✓ | ✓ active | — | — | — | the close button appears on plain `:hover` (sticks on touch) |
| Threads (`threads.css:79-80`) | plain `:hover` | **✗** | the base ring | ✓ | — | — | — | — |
| Rail handle (`shell.css:26-30`) | plain `:hover` | — | ✓ | — | — | — | — | the dragging state looks the same as hover |
| Status item, selection toolbar | ✓ | ✓ | ✓ | — | — | — | — | — |
| Toast action (`notice.css:71-88`) | **✗** | **✗** | ✓ | — | — | — | — | — |
| Link-layer anchors | **✗** (no hover look) | **✗** | ✓ (`reader.css:107-116`) | — | — | — | — | — |

- Only two places use `@media (hover: none)`: `properties.css:73` and `reader.css:822`. Both reveal on hover and show at all times on touch. `@media (hover: hover)` is used nowhere.
- `-webkit-tap-highlight-color: transparent` is set only on buttons (`button.css:14`). Toggles, tabs, swatches and rows get the iOS grey flash on tap.
- Focus rings come from one rule (`base.css:23-26`), with no transition, so they never animate. Good.
- Read-only states: no component has a `[data-readonly]` style.

## 3. Motion today

**Tokens** (`packages/tokens/tokens.json:1699-1752`, generated into `src/generated/tokens.css:70-73`). There are no easing tokens and no short UI-transition durations.

| Token | Value | Used |
| --- | --- | --- |
| `duration-toast` | 4000 ms | `overlays/toast-region.tsx:141` (`MOTION.toast`) |
| `duration-pill-fade` | 1500 ms | **unused**: no page pill exists in `packages/ui` |
| `duration-progress-cycle` | 1200 ms (0 under reduced motion, `tokens.css:639-643`) | `progress.css:39` |
| `duration-tooltip-delay` | 500 ms | `overlays/tooltip.tsx:35` |

**Every animation and transition in scope** (there are only four):

| What | Code | Issues |
| --- | --- | --- |
| Inspector grows in | `shell.css:58-68`: `animation: paper-inspector-in 160ms ease-out`, from `inline-size: 0` | Animates a **layout property**, so the canvas reflows every frame. The page re-lays out and re-renders under a resize. One way only: closing is instant (unmount). Hard-coded 160 ms and `ease-out`. Tests wait it out (`test/browser/reader.browser.test.tsx:362`, `shell.browser.test.tsx:86`). |
| Phone top bar hides | `phone.css:26-36`: `margin-block-start 160ms ease`, `visibility 0s 160ms` | Animates **margin** (layout). Both ways, and visibility is delayed correctly on hide. Hard-coded. |
| Indeterminate progress | `progress.css:39-55` | Token-driven, and stops under reduced motion. |
| "View original page" outline | `apps/web/src/app.css:130-153` (2 s `ease-out` fade) + `reader-origin.ts:8` (`OUTLINE_MS = 2000`) | The duration lives in two places. Under reduced motion the outline shows solid for 2 s, then vanishes (acceptable). |

- No `transition` on any hover, pressed or selected state. Every state change is an instant colour jump, in and out. No component transitions `all`, so nothing flashes.
- No `@keyframes` or transitions on overlays. Nothing uses React Aria's `data-entering` or `data-exiting` (grep: 0 hits). Dialogs, popovers, menus, tooltips, the palette, toasts and link previews pop in and out.
- **requestAnimationFrame**: used only for scheduling (`viewer.ts:1191`, `strip.ts:215`, `anchored.ts:47`, `dialog.tsx:207`, `sheet.tsx:379`, `session.ts:192`). The one real motion is touch fling momentum (`packages/viewer/src/view/touch.ts:228-249`, `MOMENTUM_MS = 325` at `:27`). It ignores reduced motion. That is arguably right, since native scroll momentum does the same.
- **View Transitions**: none.
- **Smooth scroll**: none. Every jump sets `scrollTop` directly (`viewer.ts:582`, `605`, `937`, `973`, `1159`; `strip.ts:209-210`), and the fling uses `behavior: "instant"` (`viewer.ts:1918`).
- **Reduced motion**: the CSS media query appears in `shell.css:58`, `phone.css:26`, `progress.css:52`, `app.css:149` and `tokens.css:639`. Settings › Reduce motion writes `data-motion="reduce"` (`reader/appearance.tsx:79`), and one `!important` layer kills all animation, transition and smooth scroll (`settings.css:217-230`). No JS reads `prefers-reduced-motion` (grep: only `pointer: coarse` at `viewer/src/view/budgets.ts:83`). The only motion test is the progress bar (`test/browser/display.browser.test.tsx:154-157`, through the `reducedMotion` Playwright command, `vitest.browser.config.ts:13-27`).
- **Hard-coded timings outside the tokens**: 160 ms ×2 and the easings (above); 2 s ×2; `PREVIEW_DWELL_MS = 300` and `PREVIEW_LONG_PRESS_MS = 500` (`viewer/src/layers/link-layer.ts:102-104`); `LONG_PRESS_MS = 500`, `TAP_MS = 250` (`touch.ts:21-25`); `SETTLE_MS = 400` (`reader/top-bar.tsx:90`); `KEEP_AFTER_MS = 250` (`shell/layout.ts:22`); `OVER_RECENTLY_MS = 250` (`reader/document-tabs.tsx:59`); `ANNOUNCE_DELAY = 700` (`reader/announcer.tsx:9`). The behavioural delays are fine as code constants. The visual ones belong in tokens.
- The design language already sets the rules: "no sliding sheets or shimmer, instant cross-fades" under reduced motion (§ checklist, `design-language.md:1371`). The sheet "moves without a slide" (`:899`). Skeletons have no shimmer (`:1035`, `progress.css:58-64`). An accent switch has "no transition under reduced motion" (`:233`), which implies one without it, but none exists.

## 4. Where delight is missing in the reader flow

- **Opening a file.** The drag overlay (`reader.css:362-374`) appears and disappears instantly. A fade of about 120 ms would soften it. The first page goes from placeholder to canvas with no fade (`viewer.ts:58`, `1039-1042`). The `low` → `sharp` swap is instant, which is correct (no motion on the large surface), but an opacity fade on the first paint only would ease it. Skeletons are flat and correct.
- **Page changes and zoom.** Page input, outline, thumbnails, find and history all jump `scrollTop` instantly. That suits large surfaces. Keep it, perhaps with a brief flash on the target page. Zoom is instant, with a stretched low-resolution bitmap in between (fine; don't animate scale).
- **Thumbnails.** No hover or press feedback at all (`app.css:83-113`). The current-page outline jumps from one thumbnail to the next. A short `outline-color` transition would help.
- **Rail and drawer.** The docked rail toggles instantly. The drawer below 1024 px (`shell.css:32-48`) and its scrim appear with no slide or fade. A transform slide of about 160 ms, plus a scrim opacity fade, is the expected motion. Under reduced motion, an instant change.
- **Inspector.** It grows by `inline-size` (layout, janky) on open only, and closes instantly. Switch it to a transform or opacity on the panel's contents, or drop it, and make it work both ways.
- **Dialogs, popovers, menus, tooltips, palette, toasts.** All instant. React Aria exposes `[data-entering]` and `[data-exiting]` (it waits for exit animations), so a 120–150 ms opacity plus a small translate or scale costs little. The palette and dialogs would gain the most. Toasts would gain a rise-in and fade-out.
- **Find bar and results.** They open instantly, and the current match moves with no emphasis. The result count updates silently, which suits screen readers (the announcer handles it).
- **Edits.** Undo and redo give a toast (`apps/web/src/edit-toasts.ts:1-5`). The History tab's current row tint jumps (`history.css:69-72`). The changed page re-renders with no cue. A brief highlight on the changed page would confirm the edit.
- **Link previews (T5a).** The preview appears after a 300 ms dwell (`link-layer.ts:102`) with no fade. It shows a gutter block while loading (`link-preview.css:30-34`) and swaps to the picture instantly. A fade-in, and a cross-fade from the gutter block to the picture, fit here.
- **Switches and toggles.** The knob jumps (`choice.css:112-119`). A 120 ms transform on the knob is the classic micro-interaction. The bold-on-select width jump in tabs and the mode switcher reads as a glitch.
- **What must not animate:** pages, the canvas, zoom and the scroll position (large surfaces). Nothing in the code is scroll-linked today. Keep it that way.

## 5. Measured in the built reader

Setup: `cargo xtask web build`, then `pnpm --filter @laminapdf/web preview` (wrangler, port 8788). Playwright Chromium 1440×900 opened `fixtures/corpus/outline.pdf` and `fixtures/links/links.pdf` by Choose a file. The script read `getComputedStyle(el).cursor` for every visible `button, a, [role=button|tab|menuitem|option|switch|checkbox|radio|slider|separator|row], input, select`. The throwaway scripts aren't kept; [U7d](../../../todos/ready/U7d-reader-surfaces-motion.md) turns this measurement into the reader's cursor test. The server was stopped afterwards.

| Screen | Total | `default` | `pointer` | Other |
| --- | --- | --- | --- | --- |
| Home (empty state) | 5 | 5 (Skip to document, View settings, Commands ⌘K, More, Choose a file) | 0 | — |
| Reader chrome, document open | 29 | 18 enabled (Sidebar, Close tab, Link tool, Zoom −/%/+, Reader mode, Find, View settings, Commands, More, Pages and Outline tabs, outline Collapse/Expand, Edit bookmarks, network status item…) + 4 disabled (Undo, Redo, Back, Forward), which look the same | 0 | `auto` ×5 (outline `role=row` items; their inner `.paper-outline-link` is a `<span>` with `default`); `text` ×1 (page input); `col-resize` ×1 (rail separator) |
| More menu | 16 menu items | 14 + 2 disabled (Undo, Redo) | 0 | — |
| Command palette | 56 options | 47 + 9 disabled | 0 | — |
| Thumbnails (Pages tab) | `div[role=option]` | `auto` (arrow) | 0 | — |
| Link-layer anchors (links.pdf) | 5 | — | 5 | — |
| Canvas | — | — | — | root `auto`, spans `text` (select tool); link tool: root `crosshair`, spans still `text` (confirms §1) |

What the run also showed:

- **Pointer cursor count in the chrome: zero.** The only pointers are PDF links in the page.
- **Hover is instant.** The Zoom in IconButton goes from `rgba(0,0,0,0)` to `rgba(31,29,26,0.06)` with `data-hovered`. Its computed `transition` is the initial `all 0s`, so nothing animates.
- **Hovering a thumbnail changes nothing.** Before and after hover, the outline is `rgb(228,222,211) 1px` and the background transparent.
- **The link preview appeared** after hovering for 700 ms with `animation: none`, `transition-duration: 0s`, `opacity: 1`.
- **The More menu popover** opened with `animation: none`, `transition-duration: 0s` and no `data-entering`.
- **The hand tool can't be reached in the web reader.** It is registered (`tools.ts:101`), but no command, toolbar or palette entry turns it on: a palette search for "hand" found nothing, and grep finds no `use("hand")` in `packages/ui/src` or `apps/web/src`. Its cursor bugs are latent until a UI exposes it.

## Top 10 problems, by how often users hit them

1. **The cursor convention on every control** (`button.css:12` and 20 more `cursor: default` rules). It is mixed with `pointer` in three places, and the design files draw `pointer`. Every click in the app meets it.
2. **No hover or pressed transitions anywhere.** Every state change is an instant colour jump, in and out. There are no easing or duration tokens to build them on.
3. **Reader thumbnails have no hover or pressed state** (`apps/web/src/app.css:83-113`). It is the rail's main list.
4. **Toggles and tool toggles have no pressed state, and lose hover when selected** (`toggle.css:22-33`). The same holds for the mode switcher and tabs, which also widen on select because of the weight change.
5. **Overlays pop in and out** (menus, popovers, tooltips, dialogs, palette, toasts, link previews). `data-entering` and `data-exiting` are never used.
6. **The link tool's crosshair turns into an I-beam over text** (measured: root `crosshair`, spans `text`; `text-layer.ts:275` vs `tools.ts:157`). The hand tool has the same bug and never shows `grabbing`, but no UI can turn it on yet.
7. **Inspector animation** runs on `inline-size` (`shell.css:58-68`). It reflows the canvas, hard-codes its timing and runs one way. The phone top bar animates `margin` (`phone.css:26-36`).
8. **Checkbox, radio, switch and swatches have no hover or pressed state**, and the switch knob jumps (`choice.css`, `swatches.css`).
9. **Plain `:hover` sticks on touch**: document tabs and their close button, history rows, threads, rail handle (`document-tabs.css:40,99-106`; `history.css:65`; `threads.css:79-80`; `shell.css:26`). No `@media (hover: hover)` guard anywhere.
10. **Small gaps:** `danger-filled` has no hover or pressed state (`button.css:94-99`), IconButton has no pending state, a hovered pending ghost button keeps its hover fill, no control has a read-only style, the drawer and scrim have no motion, `duration-pill-fade` is unused, and the origin outline's 2 s lives in two places.
