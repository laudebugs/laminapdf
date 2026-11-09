# C5b — Find in document: streaming search, highlights, options and pdf.js count parity

| Field | Value |
| --- | --- |
| ID | `C5b` |
| Status | `done` |
| Plan | [web reader §6 C5. Text, selection and search](../../plans/web-app.md#c5-text-selection-and-search) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B9c](./B9c-text-extraction-bindings-and-differential.md), [C3c](./C3c-viewer-perf-harness.md), [C5a](./C5a-text-layer-and-selection.md) |
| Dependency notes | [C5a](../done/C5a-text-layer-and-selection.md); library **B9** ([pdf-library B9](../../plans/pdf-library.md#b9-text-extraction)); [C3c](../done/C3c-viewer-perf-harness.md) (adds a find scenario to the perf harness and is measured against its budgets) |
| ADRs | Follows 0009 (find controller in the framework-free viewer; search runs in the worker at background priority), [0018](../../decisions/0018-test-strategy.md) §1 tier 4/6 (facts from independent tools: pdf.js counts) |
| Branch | `feat/c5b-find-in-document` |
| PR title | `feat(viewer): add find in document with streaming results and highlights` |
| Size | L (600–1,500 lines incl. tests and the facts script) |
| PR | [#349](https://github.com/laudebugs/laminapdf/pull/349) |

## Goal

Let users find text across a whole document without blocking scrolling: the worker searches page by page at background priority and streams matches back; the viewer highlights matches on mounted pages, keeps a running match count, and moves to next/previous matches with the keyboard; options cover case sensitivity and whole words. Match counts agree with pdf.js on the corpus, and a 1,000-page text PDF meets the §4 search targets (first result ≤ 500 ms, full pass ≤ 10 s, non-blocking).

## Context

- **Design (Paper):** [R05 Find in document](../../design/paper/reader-a.md#r05-find-in-document). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [web-app.md §4 functional 5](../../plans/web-app.md#functional) and [non-functional targets](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) (search row).
- [C5a](../done/C5a-text-layer-and-selection.md) — text runs and quads; text-layer span mapping used for highlight rectangles.
- [C3a](../done/C3a-viewer-worker-rpc.md) — priorities and cancellation (a new query cancels the old one).
- [C3c](../done/C3c-viewer-perf-harness.md) — harness to measure search timings.
- [ADR 0018 §1](../../decisions/0018-test-strategy.md#1-tiers) — expected facts produced by independent tools (pdf.js) and stored in `fixtures/corpus/expected.json`, with an exceptions file for deliberate differences.

## Scope

**In scope**

- Worker search engine `packages/viewer/src/worker/search.ts`: normalised page text (Unicode NFKC-ish folding consistent with pdf.js's normalisation: ligatures, diacritics option off by default, hyphenation at line ends, whitespace collapse) with offset maps back to glyph indices; case-insensitive and whole-word matching (Unicode word boundaries via `Intl.Segmenter`); iterate pages from the current page forward with wrap-around; yield between pages; stream `match` events `{ page, index, quads }` and `progress`; cancel on new query.
- RPC: `find(query, options) → stream`, `cancelFind`.
- `FindController` in `@laminapdf/viewer`: state (query, options, total, current index, per-page counts, pending/complete), next/previous, scroll-into-view of the current match (centering, respecting zoom), highlight layer (`lamina-layer-highlight` rectangles from quads, current match styled differently), works for unmounted pages lazily.
- `@laminapdf/ui` find bar: input, match count "3 of 120" (or "120+" while counting), previous/next buttons, case and whole-word toggles, close; `ctrl/cmd+F` opens it (intercepting the browser's find only when focus is in the reader — decide with C7a's keyboard map), `Enter`/`Shift+Enter` and `F3`/`Shift+F3` navigate, `Esc` closes and returns focus.
- Live-region announcements of result counts (polite, throttled) — wiring here, full audit in C7a.
- Facts script: `tools/` script (Node, dev-only) that runs pdf.js's find logic or text content over `fixtures/corpus` and the B9 text fixtures for a fixed query list and writes `search` facts into `expected.json` (or a sibling `search-expected.json`) with an exceptions file.

**Out of scope** (and which todo covers it)

- Regex search, search across multiple documents, search in annotations/form fields (later editor rounds).
- Full keyboard map/help → [C7a](../ready/C7a-accessibility-and-theming.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Normalisation module with unit tests (ligature `ﬁ`→`fi`, soft hyphen removal, line-end hyphen joining, NBSP→space, case folding with `toLocaleLowerCase` vs `toLowerCase` decision documented, Arabic/Hebrew/Devanagari/CJK behaviour).
- [x] Offset map from normalised string to glyph run indices so matches map to quads exactly.
- [x] Worker search loop with cooperative yielding (`await` between pages; check cancellation), reusing C5a's per-page text cache; background priority so renders preempt it.
- [x] Streaming RPC (multiple `match`/`progress` events per request) — extend the C3a protocol with a stream type if not already present.
- [x] `FindController` + highlight layer; current-match scroll; state store subscription for UI.
- [x] Find bar component in `@laminapdf/ui` and wiring in `apps/web`.
- [x] Facts script + committed facts (queries chosen to cover ligatures, hyphenation, case, whole word, RTL, CJK); exceptions file with reasons.
- [x] Perf scenario in the C3c harness: search a common word and a rare word in the 1,000-page fixture; record first-result and full-pass times; confirm no long tasks on the main thread.
- [x] Find from the phone's bottom bar (Search) on the phone and tablet profiles of `apps/web/e2e/devices.e2e.test.ts` (U3a left find to this todo).

## Acceptance criteria

- [x] **Search counts match pdf.js** for every (fixture, query) pair in the facts file, except listed exceptions with reasons.
- [x] In Playwright (three engines): typing a query shows the first highlight and a count; `Enter` cycles through matches across pages, scrolling each into view; toggling case/whole-word changes counts as expected; `Esc` closes and restores focus.
- [x] A new query cancels the previous search (no stale matches appear; asserted by event IDs).
- [ ] 1,000-page text PDF on the reference machine: first result ≤ 500 ms; full pass ≤ 10 s; no main-thread task > 50 ms while searching and scrolling (C3c harness report attached to the PR). (Measured in Chromium on a development Mac, not the reference machine, and without the full report: see Notes.)
- [x] Search works on encrypted fixtures and on remote (`#url=`) files without downloading pages the user hasn't reached until the search reaches them (range requests show progressive fetches). (Both tested end to end; the progressive fetching follows from reading each page's text as the search reaches it, but no test counts the range requests.)
- [x] Result-count changes are announced via a polite live region (no more than once per second while streaming).

## Post-merge checks

None.

## Tests

Unit tests for normalisation, offset maps and whole-word logic; Vitest browser tests for `FindController`; Playwright `apps/web/e2e/find.spec.ts` (interaction) and `find-parity.spec.ts` (counts vs facts); perf scenario in the C3c harness.

## Docs to update

`packages/viewer/README.md` (find API), `fixtures/README.md` (search facts and how they're generated), tick "Implement the find controller with streaming results, highlights and options" and the search-parity part of "Copy/extraction tests in five scripts; search-count parity tests vs pdf.js" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Normalisation choices** decide parity; pdf.js's rules evolve — pin the pdf.js version used for facts and record it.
- **Browser find (`ctrl+F`) override** is contentious; the text layer also makes native find partially work. Decide in review whether to intercept always, only when the reader has focus, or never (offer a button and `/` shortcut).
- **Memory**: caching normalised text for 1,000 pages is a few MB; fine, but evict under the worker's memory budget for huge documents.

## Notes

What the PR did differently from the plan above, and why:

- **Claim.** The work PR's first commit claimed the todo, as the repo now does, not a claim PR.
- **No stream type.** The worker answers `find(query, options, pages)` for four pages at a time, and the viewer's `FindController` asks for the next four only when an answer arrives. Results still arrive page by page, renders run between calls, and a new query stops asking and aborts the call under way; a search id drops any late answer. So the C3a protocol needed no stream type. The controller's Node tests cover the order, the wrap-around and the cancelling.
- **Folding.** `toLowerCase`, not `toLocaleLowerCase`, so a Turkish locale can't make `I` miss `i`. Diacritics are every combining mark, as pdf.js takes them: the parity facts showed pdf.js ignoring Hebrew points too.
- **Facts.** `tools/differential/search.mjs` runs pdf.js 6.4.299's own `PDFFindController` in Node over eight reference PDFs and 13 queries into `fixtures/text/search.json`. All 104 cases match, so `search-exceptions.json` is empty. The parity check is a Node test, `packages/viewer/test/find-parity.test.ts`, rather than a Playwright spec: it runs the same find code over the same text, faster. No fixture has a hyphen at a line's end, so the hyphen joining is covered by unit tests only.
- **Commands.** `find.open`, `find.next` and `find.previous` were already in the registry. ADR 0024 fixes its 39 commands, so Match case, Whole words and Close are the find bar's own buttons, with Escape in the bar, not new registry commands.
- **History.** R05 makes each Next or Previous a history jump (R07); that waits for the history in [C6b](../ready/C6b-links-and-history.md).
- **Whole words.** A match is whole when no letter, digit or mark runs on at either end, the class test pdf.js uses, rather than `Intl.Segmenter`'s word breaks: it keeps the counts equal to pdf.js's. One difference follows: Latin next to Han, with no space between, is no word boundary.
- **Names.** The worker's module is `worker/find.ts`, after the method it serves, not `worker/search.ts`. `@laminapdf/ui`'s `FindSlice` and `FindOptionsSlice` copy the shape of the viewer's `FindState` and `FindOptions`, as `ViewerSlice` does the viewer's state: the UI package never imports the viewer.
- **Ctrl+F.** The registry's existing `Mod+F` replaces the browser's find while the reader has focus, which is the whole page in this app.
- **Results tab.** R05's Results tab in the rail isn't built; the find bar carries the count, the note and next and previous. It needs the rail's tabs (C6a) and is left to a later todo.
- **Phone.** The find bar takes the same slot and wraps on narrow screens, opened from the bottom bar's Search, rather than R05's phone design (a top-bar field and a Previous · Results · Next bar).
- **Highlights.** Each match is drawn as the bounding box of its quads, so a match on rotated text is a box around it, not a rotated one.
- **Performance.** On a development Mac in Chromium, find on the 1,000-page file gave its first result in 29 ms and searched every page in 5.5 s, with no long task while the view scrolled. The harness hadn't run since U3a (#325) gave it an import Node can't resolve, which this PR fixes, but its older zoom scenario now times out waiting for sharp pages, without this PR's changes too. So the full report couldn't be attached, and the find scenario was run alone. The zoom timeout needs its own bug.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
