# T5a — Preview internal links and citations before jumping

| Field | Value |
| --- | --- |
| ID | `T5a` |
| Status | `ready` |
| Plan | [trust and proof §T5](../../plans/trust-and-proof.md#t5-long-reading) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [C6b](./C6b-links-and-history.md), [U3a](./U3a-responsive-mobile-and-touch.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) §3 (commands), §4 (the surface follows the pointer), §6 (sheets on the phone) and §8 (accessibility); [0009](../../decisions/0009-web-client-architecture.md) (one document worker, its render RPC); [0004](../../decisions/0004-render-backend.md) (bounded tiles, link geometry from Lamina PDF's model, never from the renderer); [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space); [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (no request) |
| Branch | `feat/t5a-link-previews` |
| PR title | `feat(viewer): preview internal links and citations before jumping` |
| Size | L (≈ 500–700 lines incl. tests) |
| PR | — |

## Goal

Readers of papers and manuals want to see where a link or a citation goes without losing their place. On Hacker News, 66 reader comments are about citations, link previews and a way back, a share that rose from 0.3 % to about 1 % of reader comments: Evince's hover preview is praised ([39809245](https://news.ycombinator.com/item?id=39809245)), citations in Acrobat are "very frustrating" ([39770052](https://news.ycombinator.com/item?id=39770052)), Google Scholar's reader drew 422 points for clickable citations, and Sioyek, a viewer for papers with reference previews, had three launches over 230 points. This PR shows the destination of an internal link in a preview: on hover or keyboard focus with a mouse or pen, and on long-press with touch. The preview is a crop of the destination region, drawn by the document worker's existing render RPC. A click or Enter jumps, and C6b's Back returns to the citing line.

## Context

- **Evidence:** the [readers note](../../research/hacker-news/notes/readers.md) (want 16, "Citations and link previews"; §6, recommendation 7) and the report's [R7](../../research/hacker-news/pdf-users-on-hacker-news.md#r7-serve-long-reading-reflow-a-dark-page-papers-and-manuals), which names link previews as one of the two gaps in long reading.
- **Design (Paper):** [R07 Links and history](../../design/paper/reader-a.md#r07-links-and-history) draws the preview (artboard `R07-link-preview.dc.html`) and its phone form (a sheet with **Go to page 11**); [design language §7.27](../../design/paper/design-language.md#727-tooltip-and-link-preview) specifies it as *proposed*: a 248 px card with a 92 px crop of the target page's top, shown after 300 ms, `role="tooltip"`, hoverable, Escape dismisses. Build to match; record the region preview's wider card as a deviation in R07 ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- **[C6b](./C6b-links-and-history.md)** builds what this extends: the `pageLinks(page)` RPC (link rects, `kind`, resolved destination), the link layer in `packages/viewer/src/layers/link-layer.ts` (anchors over each link, Tab order), `NavigationHistory` with `nav.back`/`nav.forward`, and the external-link dialog. Its scope draws no preview, although R07 lists C6b as the screen's todo.
- **[C6a](./C6a-thumbnails-outline-page-labels.md):** `resolveDestination`, `goToDestination(dest)` (every fit type, rotated pages), the outline RPC with titles and the current-section rule, page labels.
- **[C5a](./C5a-text-layer-and-selection.md):** the `pageText(page)` RPC (runs and quads, cached per page in the worker), used here for the crop's width and the preview's text.
- **[C3a](../done/C3a-viewer-worker-rpc.md) and [C3b](../done/C3b-viewer-virtualiser-and-zoom.md):** `ViewerClient.render(request, { priority, signal })` with `RenderRequest.clip`, the `visible` priority, late bitmaps closed by the worker client (`onDiscard`), and `RenderStats`.
- **[U3a](./U3a-responsive-mobile-and-touch.md):** the viewer's touch gestures (`touch-action: none`; long-press passes through to text selection). This todo makes a long-press on a link open the preview instead.
- **[U6e](../done/U6e-overlays-and-sheets.md):** `Popover`, `Tooltip`, `BottomSheet`, `place.ts` and the `LayerStack` behind Escape, in `packages/ui/src/overlays/`. **[U1b](./U1b-command-registry-and-palette.md):** the command registry in `packages/ui/src/commands/`.
- **Spec:** ISO 32000-2 §12.3.2 (explicit destinations `XYZ`, `Fit`, `FitH`, `FitV`, `FitR`, `FitB`, `FitBH`, `FitBV`; named destinations) and §12.5.6.5 (link annotations); [04 §1 navigation](../../research/pdf/04-interactive-security.md#1-navigation).

## Scope

**In scope**

- **What gets a preview.** Internal links from C6b's `pageLinks` (`kind: "dest"` and `"named"`):
  - a destination with a position (`XYZ` with a `top`, `FitH` and `FitBH` with a `top`, `FitR`) gets a **region preview**: a crop around that point, which is where citations land (LaTeX's hyperref writes `XYZ` with the reference's left and top);
  - a destination without one (`Fit`, `FitB`, `FitV`, `XYZ` with a null `top`) gets R07's **page preview**: the top of the page;
  - named actions (`NextPage`, `FirstPage`, …) get R07's sentence ("Goes to the next page") and no crop; `GoToR` gets R07's other-file sentence;
  - external links and unsupported actions get no preview: C6b's confirmation dialog and R07's "This link runs an action lamina doesn't run" stay as they are.
- **The region.** A pure function `previewRegion(target, page, lines)` in `packages/viewer/src/layers/link-preview.ts` returns `{ page, clip }` in ADR 0027 page space (points from the top-left of the displayed CropBox), after C6a's destination mapping:
  - the crop starts 8 pt above and 8 pt left of the destination point;
  - it ends at the right edge of the widest text line that starts within 12 pt of the destination's left and whose top lies inside the crop, so a reference in the right column of a two-column page shows that column. Without text lines (no extract module loaded, an image-only page), the crop is 360 pt wide;
  - it is at most 180 CSS px high at the preview scale, and never leaves the page;
  - for `FitR`, the rectangle itself, scaled to fit the card;
  - bad values (a `top` above the page, a null `left`, a rect outside the CropBox) are clamped and never throw (ADR 0003).
- **The scale.** The reader's current zoom clamped to 100–150 %, so preview text is no smaller than at actual size, times the device pixel ratio. The card is at most 480 × 180 CSS px: a region wider than the card at that zoom lowers the scale to fit, but never below 75 %, and past that the crop is cut at the card's right edge. The page preview uses R07's sizes (a 92 px crop of the page's top at thumbnail scale).
- **One render, through the worker.** `Viewer.preview(target, { signal })` (new, in `packages/viewer/src/view/viewer.ts`) returns `{ bitmap, width, height, text, title }`:
  - the bitmap comes from one `client.render({ page, scale, clip, rotation }, { priority: "visible", signal })`, with the view rotation applied as C6a's mapping does for rotated pages;
  - `text` is the text of the lines inside the crop, from C5a's `pageText` for the destination page, joined and cut at 300 characters;
  - `title` is the deepest outline item at or before the destination (C6a's current-section rule), else the page label ("Page 11", "A-1").
  The viewer keeps the last 8 preview bitmaps in a small LRU, closed on eviction, on a zoom change and when the document closes. A render aborted before its answer has its late bitmap closed by the worker client.
- **Preview events from the link layer.** C6b's link layer emits `linkpreview` (`{ link, anchor, rect, pointer: "fine" | "keyboard" | "touch" }`) and `linkpreviewend`:
  - **mouse and pen:** 300 ms after `pointerenter`; the end comes 300 ms after the pointer leaves both the link and the preview, so the preview can be hovered (WCAG 1.4.13);
  - **keyboard:** 300 ms after a link gets focus (`:focus-visible`); the end comes on blur or Escape;
  - **touch:** a long-press of 500 ms that moves less than 10 CSS px. The click that would follow is cancelled. A long-press on a link starts no text selection, while a long-press elsewhere still selects text (U3a). Link anchors get `-webkit-touch-callout: none`, and `contextmenu` is cancelled for touch on links, so the OS link menu stays shut. A tap still follows the link, as in C6b.
- **The surface follows the pointer** (ADR 0024 §4), drawn by `LinkPreview` in `packages/ui/src/overlays/link-preview.tsx` (exported from `@laminapdf/ui`):
  - **mouse, pen and keyboard:** a popover in the popover layer (`--paper-z-popover`), `role="tooltip"`, placed under the link and flipped above on collision (`place.ts`), never covering the link. The anchor's `aria-describedby` names it while it shows. It registers with `LayerStack`, so Escape (`layer.close`) closes it first and focus stays on the link. Enter follows the link (C6b). Content as R07: the crop on a `<canvas>` (`role="img"`, its name the region's text, or "Top of page 11" for a page preview), the title (Newsreader 15/500), and "Link to page 11 · Enter to go" in `muted`;
  - **touch:** a `BottomSheet` at the medium detent (ADR 0024 §6), with the crop, the title, the region's text, **Go to page 11** (primary, runs `nav.follow-link`) and Close. There is no floating popover on touch, and only one sheet shows at a time.
  Strings go in `packages/ui/src/messages.ts`; colours and sizes come from `--paper-` tokens only; with reduced motion nothing animates.
- **Commands** (ADR 0024 §3): register `nav.follow-link` ("Follow link", in the design language's appendix) in U1b's registry if C6b hasn't. It acts on the focused or previewed link, its visible homes are the link itself and the sheet's button, and its reason when disabled is "Focus a link first". Back after a jump is C6b's `nav.back` ("Back to page 2"). No new shortcut.
- **Measurement.** A `link-previews` scenario in the viewer's perf harness (`packages/viewer/perf/`): hover 20 citations in a row on the fixture below and record the time from dwell to crop and the longest main-thread task (reported, not asserted, as C3c's figures are).

**Out of scope** (and which todo covers it)

- Citations with no link annotation (plain "[12]" matched to a reference list by its text, as Sioyek does): a later todo if users ask; it needs a heuristic in `laminapdf-extract`.
- Scrolling, zooming or pinning a preview (Sioyek's "portals"): not planned.
- Previews of external pages: never fetched (ADR 0011 §3); C6b's dialog is the only surface.
- Hover previews in the rail (outline, thumbnails): C6a's rail already shows them.
- The desktop app: it reuses these packages; native menus are [D4b](./D4b-native-menus.md).

## Tasks

- [ ] Claim via the work PR (see Definition of done); a fresh worktree off `main`. Read C6b's and C6a's merged code first: the names below follow their todos and may have changed.
- [ ] `packages/viewer/src/layers/link-preview.ts`: `previewRegion` and the scale rule, with unit tests for every fit type, both columns of a two-column page, `/Rotate 90` and `270`, pages with a CropBox offset, clamped bad values, and pages without text.
- [ ] `Viewer.preview(target, { signal })`: the render call, the text and title, the 8-entry bitmap LRU and its closing rules; export the result type from `packages/viewer/src/index.ts`.
- [ ] The link layer's `linkpreview`/`linkpreviewend` events: dwell and grace timers, focus, long-press with the movement threshold, cancelled clicks, `-webkit-touch-callout` and `contextmenu` on touch. Check against U3a's gesture code that a long-press off a link still selects text.
- [ ] `LinkPreview` in `@laminapdf/ui`: popover and sheet by pointer, `LayerStack` registration, `aria-describedby` on the anchor, messages and tokens; its states beside the other overlays' in the package's browser tests (and in [U6f](../done/U6f-gallery-and-visual-tests.md)'s gallery once it exists).
- [ ] Wire it in the reader app (`apps/web`): the viewer's events to `LinkPreview`, `nav.follow-link` in the registry.
- [ ] The fixture: an 8-page two-column "paper" built at test time with `@laminapdf/sdk`'s `Builder`: 30 citations `[1]`–`[30]` as text runs with a `link` to named destinations `cite.1`–`cite.30`, made with `destination(name, index, fit)` at an `XYZ` position on each reference entry, split over both columns of pages 7–8; a "see page 3" link to a `Fit` destination; one page with `/Rotate 90`. Use C6b's every-link-kind fixture for named actions and `GoToR`.
- [ ] The perf scenario; R07 and §7.27 updated (below).

## Acceptance criteria

- [ ] **Hover, three engines** (Playwright, Chromium, Firefox, WebKit, a mouse): over a citation, the popover appears 300 ms (± 50) after `pointerenter`, and its crop is drawn within 500 ms more in CI. Its accessible name contains the reference entry's first line exactly as the fixture wrote it. For entries in the right column, the crop's left edge is within 8 pt of the destination's `left`.
- [ ] **Hoverable and dismissible** (WCAG 1.4.13): moving from the link onto the popover keeps it open; leaving both hides it after 300 ms; Escape hides it and focus stays on the link.
- [ ] **Jump and back:** clicking a citation brings its destination point within 16 CSS px of the viewport's top; `nav.back` returns to the citing position within 16 CSS px, and Forward goes to the reference again.
- [ ] **Keyboard:** Tab to a citation shows the preview after 300 ms, the link's `aria-describedby` names the tooltip, and Enter follows. axe reports no serious or critical violation with the popover open, in the Light, Dark and High contrast themes.
- [ ] **Touch** (Playwright with `hasTouch`: WebKit's iPhone and Chromium's Pixel profiles): a long-press on a citation opens the sheet at the medium detent, selects no text and leaves the anchor's computed `-webkit-touch-callout` at `none`. **Go to page 7** follows and Back returns. A tap follows without a sheet, and a long-press on plain text still selects text.
- [ ] **The other kinds:** a `Fit` link shows R07's page preview (the page's top); a named action shows its sentence and no crop; `GoToR` shows the other-file sentence; an external link shows no preview and still opens C6b's dialog.
- [ ] **Nothing leaks:** leaving a link within 300 ms sends no render (the client's `render-start` events show none); leaving later aborts the render. `RenderStats.drawsOnUnmounted` stays 0, and the viewer never holds more than 8 preview bitmaps (a counter checked after hovering all 30 citations).
- [ ] **No request:** with previews in use, the privacy suite's request log (C4c's `apps/web/e2e/privacy.spec.ts`) shows no new request to any origin.
- [ ] A destination on the `/Rotate 90` page previews the right region (unit test and one browser check), and an encrypted fixture previews after its password with no second prompt.

## Post-merge checks

- [ ] A long-press on a citation on a real iPhone (Safari) and an Android phone (Chrome) opens the sheet without the OS link menu (recorded in the done todo; emulation doesn't reproduce iOS's callout).

## Tests

- Vitest unit tests: `packages/viewer/src/layers/link-preview.test.ts` (geometry) and the link layer's timers with fake timers (dwell, grace, long-press threshold, movement cancel).
- Vitest browser tests: `packages/viewer/test/browser/` for `Viewer.preview` (one render, the LRU, aborts) and `packages/ui/test/browser/link-preview.browser.test.tsx` (popover or sheet by pointer, ARIA, Escape through `LayerStack`, axe in 3 themes × 2 directions, as `a11y.browser.test.tsx` does for the other overlays).
- Playwright: `apps/web/e2e/link-previews.spec.ts` in the three engines, with the touch profiles; the paper fixture built in the test's setup.
- The perf harness's `link-previews` scenario (`cargo xtask viewer-perf`).

## Docs to update

- [R07](../../design/paper/reader-a.md#r07-links-and-history): add T5a to its **Todos** line, and record the region preview (card up to 480 × 180 CSS px, crop at the reader's zoom clamped to 100–150 %) as a deviation.
- [Design language §7.27](../../design/paper/design-language.md#727-tooltip-and-link-preview): the link preview is built (no longer proposed), with the region preview's sizes.
- `packages/viewer/README.md` (link previews, `Viewer.preview`) and CLAUDE.md's `packages/viewer` entry.
- [Trust and proof §11](../../plans/trust-and-proof.md#11-to-dos): tick T5a.

## Risks and open questions

- **Destinations that point at the top of the page.** Some writers (hyperref without `hypertexnames`, office bookmarks) give every destination the page's top. The region preview then shows the page's top band, which is right for headings and wrong for references. Count how many corpus links have a positioned destination and record it in the PR; a fallback that finds the citation's text on the page would be a later todo.
- **Long-press against text selection.** A selection that starts on a link and the iOS callout are the usual conflicts; emulation misses some of them, hence the post-merge check.
- **Two parsers.** Link rects and destinations come from Lamina PDF (C6b), pixels from PDFium (ADR 0004). If the two disagree on a page box, the crop shows the wrong band; the rotated and CropBox-offset tests catch the common cases.
- **A setting to turn previews off:** recommendation no for now. The 300 ms dwell and Escape keep previews out of the way; add a View setting only if users ask.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
