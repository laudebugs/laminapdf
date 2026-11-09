# K1a — `browser-extension` spike: the reader in an extension, and PDF takeover in Chromium and Firefox

| Field | Value |
| --- | --- |
| ID | `K1a` |
| Status | `ready` |
| Plan | [Browser extension §5 K1](../../plans/browser-extension.md#k1-prove-it-decide-it-draw-it) |
| Round / milestone | Round K (browser extension) — M2b Browser extension |
| Depends on | [C7c](./C7c-performance-gates-and-launch.md) |
| Dependency notes | Round K is a nice-to-have: it waits for the web reader's v0.1 launch so it never takes time from Round C. The spike needs nothing from C7c technically. |
| ADRs | Informs ADR 0033 (written in K1b). Follows [0009](../../decisions/0009-web-client-architecture.md) (single-threaded, no COOP/COEP), [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (privacy guarantees), [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (spikes live outside both workspaces) |
| Branch | `spike/k1a-browser-extension` |
| PR title | `spike(extension): load the reader in an extension and take PDFs over in Chromium and Firefox` |
| Size | M (200–400 changed lines of spike code, plus the README with results) |
| PR | — |

## Goal

Answer the questions ADR 0033 can't be written without, with evidence from real browsers. Can the built reader run unchanged inside a Manifest V3 extension page in Chrome and Firefox? Do the takeover rules from the pdf.js extension work, and does Firefox's blocking `webRequest` redirect beat its built-in viewer? Which PDFs break when Chromium fetches them a second time? How large is the package? Can the WASM build byte for byte on ARM64, as AMO's review requires?

## Context

- [Browser extension research](../../research/browser-extension/browser-extension.md): the recommendation, the per-browser table and the open questions this spike answers.
- [Platform note §2](../../research/browser-extension/notes/platform-and-pdf-interception.md#2-pdf-interception-how-to-route-pdf-navigations-to-the-extensions-viewer): how the pdf.js extension's rules work, including the Chrome 121–127 probe.
- [Distribution note](../../research/browser-extension/notes/distribution-and-tooling.md): WXT, AMO's rebuild on Ubuntu 24.04 ARM64 with Node 24.
- `apps/web/README.md`: how the reader is built (`cargo xtask web build`) and how `#url=` opens a remote file.
- [Code organisation](../../guidelines/code-organization.md): spikes sit outside both workspaces; the README is the deliverable. Copy the README shape from `spikes/pdfium-wasm/README.md`.

## Scope

**In scope**

- `spikes/browser-extension/`: a WXT project (its own `package.json`, not a workspace member) whose viewer page loads the built `apps/web` reader.
- The extension CSP with `'wasm-unsafe-eval'`; PDFium and hayro loading in the viewer page and its worker, in Chrome and Firefox.
- Chromium takeover: the pdf.js rule set (response-header `content-type` rules, POST skipped, `Content-Disposition` handling, `application/octet-stream` with a `.pdf` name) and its probe for Chrome 121–127.
- Firefox takeover: `webRequest.onHeadersReceived` with `blocking`, tried with `pdfjs.disabled` both off and on. Also try `filterResponseData` to read the first response instead of fetching again.
- A fixture host (reuse `apps/web/e2e/remote-host.ts`) serving PDFs without CORS, behind a `SameSite` cookie, as one-time links, and with `Content-Disposition: attachment`.
- Package size with PDFium and hayro, and with hayro alone.
- Whether keeping the original `Referer`, as pdf.js does, lets cookie-gated links survive the second fetch.
- The lowest `minimum_chrome_version` that works for `browser.*` and response-header rules, checked in current Brave, Opera and Vivaldi.
- Whether a strict `connect-src` in the viewer page's CSP still lets it fetch PDFs from any site, or the fetch must move to the background script.
- Whether `use_dynamic_url` on the viewer page stops a site from detecting the extension.
- AMO's reproducible build: build the WASM modules on x86_64 and on an ARM64 Linux container, and compare SHA-256 per file.
- Safari, if a Mac is free: whether a `.pdf` URL redirect works at all in the current Safari.

**Out of scope** (and which todo covers it)

- The decision itself: [K1b](./K1b-adr-0033-browser-extension.md).
- Production code in `apps/extension`: [K2a](./K2a-extension-app-and-manual-open.md).
- Store accounts and listings: [K2c](./K2c-chromium-release.md).

## Tasks

- [ ] Create a fresh worktree off `main` and open the work PR at once; its first commit moves this todo to `ongoing/`.
- [ ] Build the reader with `cargo xtask web build`; scaffold `spikes/browser-extension/` with WXT for Chrome and Firefox.
- [ ] Point the viewer page at the reader's `dist/`, adjust only what the extension needs (base path, CSP), and record every change.
- [ ] Add the takeover code for each browser and the fixture host cases; run each case in both browsers by hand and with Playwright where it can load extensions.
- [ ] Build the WASM modules in an `ubuntu-24.04` ARM64 container with Node 24 and compare hashes with the x86_64 build.
- [ ] Write the README: method, a results table per question, package sizes, and a recommendation for ADR 0033.

## Acceptance criteria

- [ ] The README answers plan §9's questions 1 to 7 with yes, no or a measured number, and says how each was tested.
- [ ] The reader opens a PDF from a host without CORS in Chrome and in Firefox inside the spike extension.
- [ ] The package size with and without PDFium is recorded per browser build.
- [ ] The ARM64 rebuild is reported per WASM file: identical or not, and why if not.

## Post-merge checks

None.

## Tests

None beyond the spike's own runs; spikes are not in CI.

## Docs to update

- [Browser extension plan](../../plans/browser-extension.md): tick the K1a to-do; fold the answers into §9.
- CLAUDE.md: one line under the spikes list.

## Risks and open questions

- WXT may not copy the reader's worker and WASM as plain files; the spike may need a manual copy step.
- Playwright loads extensions only in its bundled Chromium; Firefox runs by hand or with `web-ext run`.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
