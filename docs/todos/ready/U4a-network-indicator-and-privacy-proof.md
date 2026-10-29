# U4a — Network indicator and CSP-backed privacy proof

| Field | Value |
| --- | --- |
| ID | `U4a` |
| Status | `ready` |
| Plan | [UI/UX §6 U4. Trust and reflow](../../plans/ui-ux.md#u4-trust-and-reflow) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C4c](./C4c-remote-files-encryption-privacy.md), [U1c](./U1c-inverted-l-shell-and-modes.md) |
| Blocks | [C7c](./C7c-performance-gates-and-launch.md) |
| ADRs | Follows 0011 (privacy and telemetry policy) and 0024 |
| Branch | `feat/u4a-network-indicator` |
| PR title | `feat(web): add the network activity indicator and privacy proof tests` |
| Size | M (200–400 lines incl. tests) |
| PR | — |

## Goal

"No upload" is now a common claim; lamina should prove it. Show a status-bar indicator of every network request since the document opened (0 for local files), tighten the CSP so the page cannot talk to other origins, and test both independently.

## Context

- [UI/UX plan §5.7](../../plans/ui-ux.md#57-trust-surfaces-r9r13); [research: differentiators](../../research/product/pdf-app-product-and-ux.md#three-differentiators-ranked) — FBI Denver alert (March 2025) on free online converters; "client-side" doesn't guarantee no requests.
- [C4c](./C4c-remote-files-encryption-privacy.md) — remote `#url=` files, the no-upload Playwright test.
- [web-app plan: privacy and security](../../plans/web-app.md#privacy-and-security) — CSP, no third-party scripts.

## Scope

**In scope**

- A request counter fed by `PerformanceObserver` (`resource` entries) in the page and a message from the service worker for requests it serves or forwards; grouped by origin and kind (app shell, WASM, document range requests).
- Status-bar item: "0 requests" or "3 requests to example.org" with a panel listing origin, count, bytes and purpose; the app's own cached assets after load are listed separately and counted as zero document traffic.
- CSP `connect-src 'self'` plus, for `#url=` documents, the single document origin (added at runtime through a worker-scoped fetch, or documented if CSP can't be widened at runtime).
- A privacy page section that explains the indicator and how to verify it with browser devtools.

**Out of scope** (and which todo covers it)

- Telemetry of any kind → [U5c](./U5c-opt-in-usage-telemetry.md).
- Desktop equivalent (Tauri has no third-party network by default) → note in D-round docs; no indicator needed unless updates run.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Implement the counter and verify it catches fetch, XHR, images, fonts, beacons and WebSocket attempts (CSP violations reported via `securitypolicyviolation`).
- [ ] Build the status item and panel in `@lamina/ui`; message strings in the catalogue.
- [ ] Playwright: open local fixtures and assert that the indicator shows 0 and that Playwright's own request log for the page after load agrees; open a `#url=` fixture and assert only that origin appears.
- [ ] Attempt a request to a third origin from the page in a test and assert the CSP blocks it and the indicator shows the violation.

## Acceptance criteria

- [ ] Indicator and Playwright's independent request log agree on every e2e flow (test).
- [ ] A forced request to another origin is blocked by CSP and surfaced in the panel (test).
- [ ] The indicator is keyboard reachable and announced by screen readers (axe; manual VoiceOver check).

## Post-merge checks

None.

## Tests

Playwright e2e in `apps/web/e2e/privacy.spec.ts`; unit tests for the counter's grouping.

## Docs to update

Privacy page; `apps/web/README.md`; tick the indicator item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Browser extensions make their own requests that the page can't see: the panel says what it measures.
- Service-worker-served requests are invisible to `PerformanceObserver` in some browsers: the worker reports them explicitly.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
