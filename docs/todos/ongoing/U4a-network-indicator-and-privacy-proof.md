# U4a — Network indicator and CSP-backed privacy proof

| Field | Value |
| --- | --- |
| ID | `U4a` |
| Status | `ongoing` |
| Plan | [UI/UX §6 U4. Trust and reflow](../../plans/ui-ux.md#u4-trust-and-reflow) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C4c](../done/C4c-remote-files-encryption-privacy.md), [U1c](../done/U1c-inverted-l-shell-and-modes.md) |
| ADRs | Follows 0011 (privacy and telemetry policy) and 0024 |
| Branch | `feat/u4a-network-indicator` |
| PR title | `feat(web): add the network activity indicator and privacy proof tests` |
| Size | M (200–400 lines incl. tests) |
| PR | — |

## Goal

"No upload" is now a common claim; lamina should prove it. Show a status-bar indicator of every network request since the document opened (0 for local files), counted from every context that can make one (page, document worker, service worker), and test it against an independent request log. The CSP stays the production policy from C4b/C4c (`connect-src 'self' https:`, needed for `#url=` files): it blocks plain-HTTP and non-web schemes, and the indicator, not the CSP, is the proof for HTTPS traffic.

## Context

- **Design (Paper):** [R13 Network activity panel](../../design/paper/reader-b.md#r13-network-activity-panel). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.7](../../plans/ui-ux.md#57-trust-surfaces-r9r13); [research: differentiators](../../research/product/pdf-app-product-and-ux.md#three-differentiators-ranked) — FBI Denver alert (March 2025) on free online converters; "client-side" doesn't guarantee no requests.
- [C4c](../done/C4c-remote-files-encryption-privacy.md) — remote `#url=` files, the no-upload Playwright test.
- [web-app plan: privacy and security](../../plans/web-app.md#privacy-and-security) — CSP, no third-party scripts.

## Scope

**In scope**

- A request counter with three feeds, grouped by origin and kind (app shell, WASM, document range requests): `PerformanceObserver` (`resource` entries) in the page; the document worker, which at start-up wraps its global `fetch`, `XMLHttpRequest` and `WebSocket` (so no code path in it can bypass reporting) and forwards every request (the `#url=` range requests from C4c among them) and its own `securitypolicyviolation` events over the worker RPC, since the page sees neither; and the service worker, which reports requests it serves or forwards once one is registered (C7b; until then the feed is absent and the panel says so).
- Status-bar item showing **document traffic** ("0 requests" or "3 requests to example.org") with a panel listing origin, count, bytes and purpose. **App assets** (same-origin requests for `/assets/*` hashed files and the WASM modules, including the lazy PDFium load during file selection) are a separate, labelled group that doesn't count as document traffic; the classifier is one exported function used by both the indicator and the tests.
- CSP unchanged from C4b/C4c (`connect-src 'self' https:` in production). Investigate narrowing for local-only sessions (a `<meta>` policy can only add restrictions, and workers take the CSP of their own script response) and record the result; don't widen anything at runtime.
- A privacy page section that explains the indicator and how to verify it with browser devtools.

**Out of scope** (and which todo covers it)

- Telemetry of any kind → [U5c](../ready/U5c-opt-in-usage-telemetry.md).
- Desktop equivalent (Tauri has no third-party network by default) → note in D-round docs; no indicator needed unless updates run.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Implement the three feeds; in the page, wrap `WebSocket` and `navigator.sendBeacon` too (WebSockets produce no Resource Timing entries); verify fetch, XHR, images, fonts, beacons and WebSocket attempts are caught from the page and the document worker, and that worker CSP violations reach the panel.
- [ ] Build the status item and panel in `@laminapdf/ui`; message strings in the catalogue.
- [ ] Playwright: open local fixtures and assert that the indicator shows 0 and that Playwright's browser-context request log (which includes worker requests) agrees; open a `#url=` fixture and assert the worker's range requests are counted and only that origin appears.
- [ ] In a test, attempt from the worker a raw `fetch` that doesn't use any lamina helper (it must still be counted), then a plain-HTTP request (CSP blocks it; the panel shows the violation) and an HTTPS request to a third origin from the page and from the worker (CSP allows it; the indicator must count it).

## Acceptance criteria

- [ ] On every e2e flow, the indicator's total equals Playwright's browser-context request log count, and its document-traffic count equals the log filtered by the same classifier (same-origin app assets removed), including `#url=` documents whose bytes the worker fetches and cold-cache runs where PDFium loads after open (test).
- [ ] A forced plain-HTTP request is blocked by CSP and shown as a violation; a forced HTTPS request to a third origin, from the page or the worker, is counted (tests).
- [ ] The indicator is keyboard reachable and announced by screen readers (axe; manual VoiceOver check).

## Post-merge checks

None.

## Tests

Playwright e2e in `apps/web/e2e/privacy.spec.ts`; unit tests for the counter's grouping.

## Docs to update

Privacy page; `apps/web/README.md`; tick the indicator item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Browser extensions make their own requests that the page can't see: the panel says what it measures.
- Service-worker-served requests are invisible to `PerformanceObserver` in some browsers: the service worker reports them explicitly once C7b registers it.
- A future code path that fetches from a worker without reporting would make the indicator under-count: the Playwright comparison on every flow is the guard, and new fetch sites go through one reporting `fetch` wrapper.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
