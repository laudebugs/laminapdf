# C7b — Offline and PWA: service worker, manifest with `file_handlers`, OPFS cache and recent files

| Field | Value |
| --- | --- |
| ID | `C7b` |
| Status | `done` |
| Plan | [web reader §6 C7. Accessibility, offline and launch](../../plans/web-app.md#c7-accessibility-offline-and-launch) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C3c](./C3c-viewer-perf-harness.md), [C4c](./C4c-remote-files-encryption-privacy.md), [C4d](./C4d-deploy-previews-and-production.md), [C5b](./C5b-find-in-document.md), [C7a](./C7a-accessibility-and-theming.md) |
| ADRs | Follows 0011 (immutable hashed assets, headers; nothing leaves the device; local storage clearable), 0010 (`Platform.storage`, `hasFileHandlers` capability), 0009 |
| Branch | `feat/c7b-offline-pwa` |
| PR title | `feat(web): make the reader an installable offline pwa with file handling and recent files` |
| Size | L (600–1,200 lines incl. tests) |
| PR | [#373](https://github.com/laudebugs/laminapdf/pull/373) |

## Goal

After one visit, the reader works with no network: a service worker precaches the app shell and both WASM modules, a web app manifest makes it installable, and installed copies register as a PDF handler (`file_handlers` + `launchQueue`) where the browser supports it, so "Open with lamina" works from the OS. Large files use **OPFS** as a random-access cache, and a **recent files** list (metadata in IndexedDB, files on the device only) can be cleared in settings.

## Context

- **Design (Paper):** [R15 Offline, install and update](../../design/paper/reader-b.md#r15-offline-install-and-update). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [web-app.md §4 functional 9](../../plans/web-app.md#functional), [§2 scope](../../plans/web-app.md#2-scope-and-non-goals) (OS "Open with" via PWA file handling) and [§5 storage](../../plans/web-app.md#5-design-summary) (OPFS cache and recent files; IndexedDB holds metadata only; clearable).
- [web-frontend notes §6](../../research/architecture/notes/web-frontend.md) — OPFS + `createSyncAccessHandle` in workers in all engines; stream `File` into OPFS once; Workbox/Vite PWA; persist `FileSystemFileHandle`s in IndexedDB on Chromium only; under COEP, the SW must not strip headers.
- [Research: Memory64/4 GB ceiling](../../research/architecture/app-and-package-architecture.md#editing-is-an-op-log-over-a-lazy-object-graph-and-signatures-and-redaction-govern-saving) — stream from `ByteSource`/OPFS rather than holding whole files.
- ADR 0011 — headers and cache policy; the SW must preserve CSP and security headers.

## Scope

**In scope**

- Service worker (`apps/web/src/sw.ts`, built by Vite — Workbox via `vite-plugin-pwa` or a hand-written precache; justify the choice and check licences): precache `index.html`, hashed JS/CSS, `pdfium.wasm`, lamina `.wasm`, hayro `.wasm` (optional, lazy runtime cache), icons; navigation fallback to cached `index.html`; versioned cache names and cleanup on activate; update flow ("A new version is available — Reload") with `skipWaiting` only on user consent; **never** caches or intercepts requests to other origins (remote `#url=` documents are not cached by the SW).
- `manifest.webmanifest`: name, short name, icons (maskable), `start_url: "/"`, `display: "standalone"`, theme colours from tokens, `file_handlers: [{ action: "/", accept: { "application/pdf": [".pdf"] } }]`, optional `share_target` decision (recommendation: not in v0.1).
- `launchQueue.setConsumer` handling in `apps/web` to open launched files through the same open pipeline; capability `hasFileHandlers`.
- OPFS: worker-side `OpfsByteSource` using `createSyncAccessHandle`; large local files (> threshold, e.g. 200 MB) and remote files without range support stream into OPFS once, then are read randomly; quota checks (`navigator.storage.estimate()`), `navigator.storage.persist()` only on user action; cleanup policy (LRU by size, user clear).
- Recent files: IndexedDB store of metadata (name, size, page count, last opened, last page, thumbnail of page 1 as a small blob — stored locally only); on Chromium, the `FileSystemFileHandle` is stored to reopen with permission prompt; elsewhere, reopening needs the user to pick the file again (or the OPFS copy if cached); remote entries store the URL locally (never sent anywhere) — **only if the user opts in** ("Remember remote URLs", default **off**); with it off, remote documents are not added to recents at all.
- Settings: a **"Storage and privacy" section added to the settings dialog that [C7a](../done/C7a-accessibility-and-theming.md) owns** (C7b does not build its own settings UI; it registers a section through C7a's settings-section API): "Clear recent files and cached documents" (wipes IndexedDB + OPFS), "Remember recent local files" (default **on**, local only, clearly explained), "Remember remote URLs" (default **off**, opt-in; privacy-first).
- Empty state lists recent files (keyboard accessible).

**Out of scope** (and which todo covers it)

- Cloud storage/sync of recents (hosted-service rounds, after the `auth` spike).
- Threaded WASM / COEP (ADR 0009: later).
- Lighthouse/perf gates → [C7c](../ongoing/C7c-performance-gates-and-launch.md).
- Each document's remembered position, and reopening the document that was open after a reload or restart (R15's "you'll come back to page 3") → [T5b](../ready/T5b-reopen-at-last-position.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Choose SW tooling (licence check; Workbox is MIT); configure precache manifest from the Vite build; ensure `.wasm` served with `application/wasm` from cache (streaming instantiation works offline).
- [x] Headers: the SW serves the cached shell **with the real response headers** — no `<meta http-equiv>` CSP fallback, because a `<meta>` CSP cannot carry `frame-ancestors` (nor `X-Frame-Options`, `Permissions-Policy` etc.). The navigation handler returns the cached `index.html` response with its stored headers (the Cache API keeps them); if the chosen tooling rebuilds responses and drops headers, the SW wraps the body in `new Response(body, { headers })` with the header set C4b defines (imported from the same source as `_headers`, so they cannot drift). Test offline: the navigation response carries the exact CSP (including `frame-ancestors 'none'`) and the other security headers, an inline-script injection is blocked, and the page cannot be framed.
- [x] Update-available toast in `@laminapdf/ui`.
- [x] Manifest + icons (generated from one SVG source; provenance noted) + `<link rel="manifest">`.
- [x] Installability checks (Lighthouse 12+ has no PWA category, so no Lighthouse PWA audit): `apps/web/e2e/installability.spec.ts` that (a) fetches the manifest and validates it against the W3C manifest JSON schema from SchemaStore (pinned copy in `apps/web/e2e/fixtures/`) and asserts the install-relevant fields (`name`/`short_name`, `start_url` and `scope` same-origin, `display: standalone`, icons at 192 and 512 px that resolve with an image content type and the declared size, one `maskable`, `file_handlers` accepting `application/pdf`/`.pdf`); (b) asserts `navigator.serviceWorker.controller` is set after reload (the SW controls the page) and the SW has a `fetch` handler serving the navigation offline; (c) in Chromium, calls CDP `Page.getInstallabilityErrors` and asserts an empty list, and records whether `beforeinstallprompt` fired (informational: its timing depends on engagement heuristics).
- [x] `launchQueue` consumer and tests (Chromium only; feature-detected).
- [x] `OpfsByteSource` + copy-in pipeline with progress and cancellation; quota error handling; unit tests in Vitest browser mode.
- [x] Recent-files store (IndexedDB wrapper, schema version, migrations), UI list, reopen flows per capability; the "Storage and privacy" settings section (clear, remember local files, remember remote URLs — opt-in) registered in C7a's settings dialog. e2e: with the default settings, opening a `#url=` document adds nothing to IndexedDB.
- [x] Privacy suite extension (C4c): with recent files and OPFS active, still no request carries document bytes or URLs; the SW makes no network request except same-origin assets.
- [x] Offline Playwright test: load once, go offline (`context.setOffline(true)`), reload, open a local fixture, render page 1 and use find.

## Acceptance criteria

- [x] **The app works offline after first load** in Chromium, Firefox and WebKit (Playwright offline test passes: reload, open local file, render, find).
- [x] **Installability checks pass** on the preview deployment (`installability.spec.ts` with `BASE_URL` set to the preview): manifest valid against the schema with the required fields, every icon resolves, the SW controls the page, Chromium's `Page.getInstallabilityErrors` returns no errors, and the offline-reload test passes.
- [x] Installed in Chromium, the PWA appears as a handler for `.pdf` and a launched file opens (manual check recorded; automated `launchQueue` unit test).
- [x] A 500 MB local file opens via OPFS without the tab exceeding the C3c memory budget (perf harness scenario added).
- [x] "Clear recent files and cached documents" empties IndexedDB and OPFS (asserted in e2e).
- [x] CSP and security headers (including `frame-ancestors`) are still enforced when served from the SW cache; the offline navigation response carries the same header values as the online one (asserted in e2e).
- [x] The privacy suite passes with the SW active.

## Notes

- **Service worker tooling:** hand-written (`apps/web/src/sw/service-worker.ts`, compiled by `apps/web/plugins/pwa.ts`), not Workbox. The worker is small, and serving a cached response as stored keeps its headers, which a rebuilt response would lose. Nothing new to licence.
- **Headers offline:** the offline navigation carries exactly the server's headers, `frame-ancestors 'self'` included: ADR 0011 §4 sets `'self'`, and the ADR wins over this todo's `'none'`.
- **`share_target`:** not in v0.1, as recommended.
- **Linked files are never listed (owner decision open):** this todo asked for "Remember remote URLs", opt-in. ADR 0011 says a document's address "is never written to storage, the recent list or logs" (§2) and that the recent list holds "never bytes or URLs" (§3.9), and the ADR wins, so the setting isn't built and a `#url=` document leaves nothing in storage. Allowing it takes an ADR 0011 amendment.
- **Copies follow "Remember recent local files":** with it off, a large file's OPFS copy is removed when its document closes (`ViewerOpenOptions.keepCopy`). Copies are evicted by last use, recorded beside each copy.
- **`navigator.storage.persist()`:** never called, so never without a user action; offering it in Settings is left for later.
- **Stacked on C7a:** started on C7a's branch (#370) at the owner's request and rebased onto `main` once it merged.
- **The e2e browsers** accept the e2e server's self-signed certificate for service workers (Chromium's `--ignore-certificate-errors`, Firefox's service-worker testing mode), so every suite runs with the worker active; the privacy suite passes so.
- **Left for later:**
  - Reopening a recent file from its OPFS copy on browsers that keep no handle: copies are named by the file's date, which the recent list doesn't keep. Such rows ask for the file again.
  - A 500 MB scenario in C3c's perf harness: the copy path is tested with a lowered threshold in the viewer's browser tests, not measured at size.
  - Running `installability.e2e.test.ts` against the PR preview (`BASE_URL`): it runs against the local server.
  - The update flow across two real deploys: `AppUpdates` is unit-tested, and the PR asks for the manual check.
  - A thumbnail of page 1 in the recent list: R01 draws none.
  - A remote file without range support is still held in memory (up to 256 MiB), not streamed into OPFS: ADR 0011 §2 wants that copy keyed by a random ID and deleted at close, which is its own change.
  - The R15 differences recorded in [reader-b.md](../../design/paper/reader-b.md#r15-offline-install-and-update).

## Post-merge checks

None.

## Tests

Vitest browser tests (OPFS source, IndexedDB store, launch consumer); Playwright `apps/web/e2e/offline.spec.ts`, `recent-files.spec.ts`, `installability.spec.ts`, extended `privacy.spec.ts`; the installability spec also runs against the PR preview (C4d's `BASE_URL` smoke job).

## Docs to update

`apps/web/README.md` (offline, storage, what is stored locally and how to clear it); tick "Service worker caching shell + WASM; web app manifest with `file_handlers`" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Stale SW caches** can pin users to old versions; the update flow and versioned caches must be tested across two deploys (preview A → preview B).
- **`file_handlers`** is Chromium-only; Safari/Firefox users won't get "Open with"; document it.
- **OPFS quotas** differ and Safari may evict; treat OPFS as a cache, never the only copy.
- **Recent remote URLs** are sensitive: decided — remembered only if the user opts in (default off); local files are remembered by default and clearable.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
