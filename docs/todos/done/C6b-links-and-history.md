# C6b — Link layer (internal, named and external links) and back/forward history

| Field | Value |
| --- | --- |
| ID | `C6b` |
| Status | `done` |
| Plan | [web reader §6 C6. Navigation, print and properties](../../plans/web-app.md#c6-navigation-print-and-properties) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C5a](./C5a-text-layer-and-selection.md), [C6a](./C6a-thumbnails-outline-page-labels.md) |
| ADRs | Follows 0004 (link geometry from lamina's model, not the renderer), 0009 (framework-free link layer), 0010 (`Platform.openExternal`), 0011 (no referrer, external links confirm before leaving) |
| Branch | `feat/c6b-links-and-history` |
| PR title | `feat(viewer): add the link layer with external-link confirmation and navigation history` |
| Size | M–L (500–1,000 lines incl. tests) |
| PR | [#357](https://github.com/laudebugs/laminapdf/pull/357) |

## Goal

Make links in PDFs work safely. A DOM link layer on each mounted page turns link annotations into keyboard-focusable anchors: internal destinations and named destinations jump within the document, `URI` actions to the web ask for confirmation first (showing the full target) and open without a referrer, and other actions (`Launch`, `JavaScript`, `SubmitForm`…) are never executed. A back/forward history records every jump (links, outline, go-to-page, find) so readers can return where they were, integrated with the browser's own Back button through the URL fragment.

## Context

- **Design (Paper):** [R07 Links and history](../../design/paper/reader-a.md#r07-links-and-history). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §1 navigation](../../research/pdf/04-interactive-security.md#1-navigation) — link annotations (ISO 32000-2 §12.5.6.5), actions (§12.6.4: GoTo, GoToR, URI, Named, Launch, JavaScript), named destinations (`/Dests` name tree, §12.3.2.4).
- [web-app.md §2 scope](../../plans/web-app.md#2-scope-and-non-goals) — external links confirm before leaving; PDF JavaScript is never executed.
- [web-app.md §4 functional 6](../../plans/web-app.md#functional) and [privacy and security](../../plans/web-app.md#privacy-and-security) (`Referrer-Policy: no-referrer`).
- [C6a](../done/C6a-thumbnails-outline-page-labels.md) — destination resolution and mapping.
- [C3b](../done/C3b-viewer-virtualiser-and-zoom.md) — `lamina-layer-annot` slot.

## Scope

**In scope**

- RPC `pageLinks(page) → Link[]` (rect/quads in PDF space, `kind: "dest" | "named" | "uri" | "remote" | "named-action" | "unsupported"`, resolved destination or URI, `/Contents` alt text, border style ignored in the layer) from lamina's annotation reading; add a minimal read-only link-annotation accessor in `laminapdf-model`/`laminapdf-wasm` if B2/B6 lack it (coordinate with library owner).
- `packages/viewer/src/layers/link-layer.ts`: `a` elements positioned over link rects (above the text layer, below highlights; links don't block text selection — pointer events only on the link rect), `href` for internal links built from the **current fragment** with C4c's `buildFragment(current, { page: N })` (e.g. `#url=<encoded>&page=N`), so `#url=` and other keys are preserved and middle-click/new-tab on a remote document opens the same document at that page (for a local file the new tab shows the empty reader — there is no way to hand a `File` to another tab; acceptable), and a primary click/Enter always `preventDefault()`s and routes through `NavigationHistory`; for external links `href` is the URI, `aria-label` from `/Contents` or the URI/destination; Tab order follows page order.
- External link confirmation dialog in `@laminapdf/ui` ("This document wants to open https://… — Open / Copy link / Cancel"), showing the full URL with the host emphasised; only `http(s):` and `mailto:` allowed, everything else shown as unsupported; opened via `Platform.openExternal` with `noopener,noreferrer`.
- Named actions (`NextPage`, `PrevPage`, `FirstPage`, `LastPage`) supported; `GoToR` (remote file) shown as unsupported with the file name; `Launch`, `JavaScript`, `SubmitForm`, `ImportData`, `ResetForm` never executed (logged in debug, listed in the "unsupported features" notice).
- `NavigationHistory` in `@laminapdf/viewer`: stack of `{ page, scroll offset, zoom }` with coalescing (scrolling alone doesn't push; jumps do), back/forward methods, max length; keyboard `alt+←`/`alt+→` (and `cmd+[`/`cmd+]` on macOS); toolbar buttons; integration with `history.pushState` using only the fragment, built with `buildFragment(current, { page: N })` so `url` and other keys are preserved (never a bare `#page=N`), and relying on C4c's `hashchange` rule (only a changed `url` re-opens), so browser Back works without leaking anything to the server.

**Out of scope** (and which todo covers it)

- Form widget links and annotation interactivity beyond links (editor rounds).
- Full keyboard-shortcut map → [C7a](./C7a-accessibility-and-theming.md).
- Link previews on hover, focus and long-press (R07's preview card and its phone sheet) → [T5a](../ongoing/T5a-link-previews.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Library accessor for link annotations and actions if missing (Rust unit tests; tolerant to malformed `/A`, `/Dest`, and URI encodings per ADR 0003).
- [x] RPC + worker caching per page.
- [x] Link layer with transforms shared with the text layer (same matrix helper), hover cursor, focus outline, high-contrast-visible focus.
- [x] Confirmation dialog, URL normalisation and display (punycode shown for IDN hosts to prevent spoofing), scheme allow-list.
- [x] Named actions and unsupported-action reporting.
- [x] `NavigationHistory` with unit tests; toolbar back/forward buttons; fragment integration and `popstate` handling.
- [x] Facts: link targets (page index for internal, URI for external) for corpus files added to `expected.json` via an independent tool (pypdf/pdf.js); a generated fixture with every link kind (`@laminapdf/sdk` builder supports links and named destinations per B5).

## Acceptance criteria

- [x] **Link targets match `expected.json`** for every corpus file with links (internal page index and external URI).
- [x] Clicking or pressing Enter on an internal link navigates to the destination with the right fit; Back returns to the previous position (page and scroll within 1 page height), Forward re-applies.
- [x] Browser Back after an internal jump returns to the previous page without reloading or re-opening (no new `open` RPC, no new remote requests); the URL path never changes, only the fragment; `#url=` stays intact.
- [x] Internal link `href`s on a remote document keep the encoded `url` key (`#url=<encoded>&page=N`); a middle-click (`page.click(…, { button: "middle" })`, or reading the `href` and opening it in a new page where the engine doesn't open tabs on middle-click) opens a new tab on the same remote document at page N; a primary click doesn't change the fragment except through `NavigationHistory`.
- [x] External link: a confirmation shows the full URL; Cancel does nothing; Open opens a new tab with no `Referer` (asserted with a local test server) and no `window.opener`.
- [x] `javascript:`, `file:`, `data:` URIs and `Launch`/`JavaScript` actions are never executed; they're reported as unsupported.
- [x] Links are reachable by Tab in page order and have accessible names; zero serious axe violations on a links fixture (Tab tested in Chromium and Firefox; see Notes).
- [x] Text under a link can still be selected by dragging from outside the link.

## Post-merge checks

None.

## Tests

Rust unit tests (accessor); Vitest unit tests (history coalescing, URL validation and display); Vitest browser tests (link-layer DOM); Playwright `apps/web/e2e/links.spec.ts` (including middle-click/new-tab on an internal link of a `#url=` document) and `history.spec.ts` (three engines) with a local server checking `Referer`.

## Docs to update

`packages/viewer/README.md`, `fixtures/README.md`; tick "Link layer: internal destinations, URIs with confirmation, named destinations" and "Back/forward navigation history" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Spoofing**: link text in the page can claim one URL while `/URI` points elsewhere; the dialog always shows the real target — make that explicit in copy.
- **Overlapping links and text selection** conflicts in Safari; test drag-selection starting on a link.
- Whether to show a "don't ask again for this site" option — recommendation: no in v0.1 (privacy-first, no persistence of browsing targets).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **The fixture sits apart from the corpus.** `fixtures/corpus/` has no file with links, and a file added there would need render goldens, differential facts and text facts. So `fixtures/links/links.pdf` has every link kind, and pypdf's facts are in `fixtures/links/expected.json`. The first acceptance criterion holds on that file and on the links-and-outlines golden, whose pypdf facts were already in `fixtures/build/facts.json`. pypdf 6.21 reads either the catalog's `/Dests` or the `/Names` tree, never both, so `tools/fixtures/link_facts.py` reads the tree as well.
- **The browser's history is the history.** Each jump is a browser history entry, marked with the viewer's entry id, so the browser's Back, Alt+← and ⌘[ (which the web reader leaves to the browser) and the reader's own buttons all step one history. An entry from another view (a reload, a backend switch) goes by its fragment's page instead.
- **Selection over links.** Anchors take pointer events, so on their own they block a drag that crosses them. While a drag that started outside every link is under way, the layer lets the pointer through them. In Firefox, no drag selected anything once the page box had had the focus; that was a bug of its own ([firefox-selection-after-a-field](../../bugs/firefox-selection-after-a-field/detail.md)), and since its fix the e2e test of that drag runs in all three engines.
- **Web links have no `href`.** A browser's own menu ("Open link in new tab") would open an `href` without asking, so a web link's anchor has none. Its title shows the address, and the layer follows it on a click, a middle click or Enter, always through the confirmation.
- **Tab in WebKit.** Safari's Tab skips links unless the user turns on "Press Tab to highlight each item", so the browser test of Tab order is skipped there; the anchors are in the Tab order all the same.
- **No new open.** The e2e test checks that Back and Forward make no new request for the file; the worker's own `openDocument` calls aren't counted, but a reopen would read the file again.
- **Debug log.** Under `#debug`, the web reader logs each link it doesn't follow, with the note it shows (`onInert`).
- **Referer.** WebKit sends an empty `Referer` header where the others send none. Either way, the site learns nothing.
- **Generated types.** tsify writes a flattened enum as `interface … extends` a union, which TypeScript refuses, so `PageLinkJs`'s type is written by hand in `laminapdf-wasm`.
- **What differs from R07** is in its "As built" note: link names without the outline title, no Not run listing yet, no link commands, and no Back or Forward on the phone yet.
