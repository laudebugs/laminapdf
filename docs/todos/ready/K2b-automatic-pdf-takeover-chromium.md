# K2b — Open PDFs automatically in Chromium

| Field | Value |
| --- | --- |
| ID | `K2b` |
| Status | `ready` |
| Plan | [Browser extension §5 K2](../../plans/browser-extension.md#k2-chromium-chrome-edge-brave-vivaldi-opera) |
| Round / milestone | Round K (browser extension) — M2b Browser extension |
| Depends on | [K2a](./K2a-extension-app-and-manual-open.md) |
| ADRs | Implements 0033's takeover and permission sections |
| Branch | `feat/k2b-pdf-takeover-chromium` |
| PR title | `feat(extension): open PDFs automatically in Chromium, with a fallback to the browser's viewer` |
| Size | M (200–400 changed lines: rules, onboarding, fallback and tests) |
| PR | — |

## Goal

Let the user choose to open every PDF in Lamina PDF. The onboarding page offers automatic opening and asks for access to all sites only when the user accepts. Declarative rules then send PDF responses to the extension's viewer, as the pdf.js extension does. Every takeover keeps the browser's own viewer one click away. When the extension's fetch fails, as one-time links do on Chromium's second fetch, it falls back to that viewer on its own. Links to a page (`#page=`) open at that page, which Adobe's extension gets wrong.

## Context

- [Platform note §2](../../research/browser-extension/notes/platform-and-pdf-interception.md#2-pdf-interception-how-to-route-pdf-navigations-to-the-extensions-viewer): the pdf.js rule set and its edge cases.
- K1a's README: which links failed the second fetch, and the Chrome 121–127 probe.
- K1c's area document: the onboarding page, settings and the fallback notice.
- [Competitors note](../../research/browser-extension/notes/competitors-and-demand.md): Adobe's documented takeover bugs, which are this todo's test cases.

## Scope

**In scope**

- Dynamic declarativeNetRequest rules for `main_frame` and `sub_frame` PDF responses, with the probe that turns them off where the response-header condition is ignored.
- Onboarding and settings: automatic opening on or off, `permissions.request` for access to all sites, `file://` access explained.
- The fallback: a control in the reader that reopens the PDF in the browser's viewer, and automatic fallback on a failed fetch.
- `#page=` and `#nameddest=` carried from the original URL.
- Revoked access detected at start-up (`permissions.contains`), turning automatic opening off and saying so.

**Out of scope** (and which todo covers it)

- Embedded PDFs (`<embed>`, `<object>`): later ([plan §10](../../plans/browser-extension.md#10-later)).
- Firefox's takeover: [K3a](./K3a-firefox-extension.md).

## Tasks

- [ ] Port the pdf.js rules into the background script, with tests for each rule's matching.
- [ ] Build onboarding, settings and the fallback notice from K1c.
- [ ] Add the fallback paths and the `#page=` handling.
- [ ] Extend the Playwright suite with the fixture host's cases: no CORS, cookie-gated, one-time link, attachment, POST, octet-stream with a `.pdf` name.
- [ ] Try each case in Chrome and Edge by hand.

## Acceptance criteria

- [ ] With automatic opening on, a PDF link on a host without CORS opens in Lamina PDF at the page its `#page=` names.
- [ ] A one-time link falls back to the browser's viewer and the user sees the document.
- [ ] With automatic opening off, the extension holds no host permission.

## Post-merge checks

None.

## Tests

Playwright cases above; unit tests for the rule builder.

## Docs to update

- `apps/extension/README.md` and the plan (tick K2b).

## Risks and open questions

- Some sites send PDFs only to requests with their own cookies; K1a's results decide whether a Referer or credentials rule is needed.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
