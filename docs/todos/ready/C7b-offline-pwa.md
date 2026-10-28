# C7b — Offline and PWA: service worker, manifest with `file_handlers`, OPFS cache and recent files

| Field | Value |
| --- | --- |
| ID | `C7b` |
| Status | `ready` |
| Plan | [web reader §6 C7. Accessibility, offline and launch](../../plans/web-app.md#c7-accessibility-offline-and-launch) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C4c](./C4c-remote-files-encryption-privacy.md), [C4d](./C4d-deploy-previews-and-production.md) |
| Blocks | [C7c](./C7c-performance-gates-and-launch.md) |
| ADRs | Follows 0011 (immutable hashed assets, headers; nothing leaves the device; local storage clearable), 0010 (`Platform.storage`, `hasFileHandlers` capability), 0009 |
| Branch | `feat/c7b-offline-pwa` |
| PR title | `feat(web): make the reader an installable offline pwa with file handling and recent files` |
| Size | L (600–1,200 lines incl. tests) |
| PR | — |

## Goal

After one visit, the reader works with no network: a service worker precaches the app shell and both WASM modules, a web app manifest makes it installable, and installed copies register as a PDF handler (`file_handlers` + `launchQueue`) where the browser supports it, so "Open with lamina" works from the OS. Large files use **OPFS** as a random-access cache, and a **recent files** list (metadata in IndexedDB, files on the device only) can be cleared in settings.

## Context

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
- Recent files: IndexedDB store of metadata (name, size, page count, last opened, last page, thumbnail of page 1 as a small blob — stored locally only); on Chromium, the `FileSystemFileHandle` is stored to reopen with permission prompt; elsewhere, reopening needs the user to pick the file again (or the OPFS copy if cached); remote entries store the URL locally (never sent anywhere) — with a setting to disable remembering remote URLs.
- Settings: "Clear recent files and cached documents" (wipes IndexedDB + OPFS), "Remember recent files" toggle (default on? — decide; recommendation: on, local only, clearly explained).
- Empty state lists recent files (keyboard accessible).

**Out of scope** (and which todo covers it)

- Cloud storage/sync of recents (hosted-service rounds, after the `auth` spike).
- Threaded WASM / COEP (ADR 0009: later).
- Lighthouse/perf gates → [C7c](./C7c-performance-gates-and-launch.md).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`.
- [ ] Choose SW tooling (licence check; Workbox is MIT); configure precache manifest from the Vite build; ensure `.wasm` served with `application/wasm` from cache (streaming instantiation works offline).
- [ ] Headers: confirm the SW returns cached responses with the original security headers (CSP etc.) or that `index.html` CSP comes from a `<meta http-equiv>` fallback where needed — test that CSP is still enforced offline.
- [ ] Update-available toast in `@lamina/ui`.
- [ ] Manifest + icons (generated from one SVG source; provenance noted) + `<link rel="manifest">`.
- [ ] `launchQueue` consumer and tests (Chromium only; feature-detected).
- [ ] `OpfsByteSource` + copy-in pipeline with progress and cancellation; quota error handling; unit tests in Vitest browser mode.
- [ ] Recent-files store (IndexedDB wrapper, schema version, migrations), UI list, reopen flows per capability, clear/disable settings.
- [ ] Privacy suite extension (C4c): with recent files and OPFS active, still no request carries document bytes or URLs; the SW makes no network request except same-origin assets.
- [ ] Offline Playwright test: load once, go offline (`context.setOffline(true)`), reload, open a local fixture, render page 1 and use find.

## Acceptance criteria

- [ ] **The app works offline after first load** in Chromium, Firefox and WebKit (Playwright offline test passes: reload, open local file, render, find).
- [ ] **Lighthouse PWA installability passes** on the preview deployment (manifest valid, SW controls the page, icons present) — report attached to the PR.
- [ ] Installed in Chromium, the PWA appears as a handler for `.pdf` and a launched file opens (manual check recorded; automated `launchQueue` unit test).
- [ ] A 500 MB local file opens via OPFS without the tab exceeding the C3c memory budget (perf harness scenario added).
- [ ] "Clear recent files and cached documents" empties IndexedDB and OPFS (asserted in e2e).
- [ ] CSP and security headers are still enforced when served from the SW cache.
- [ ] The privacy suite passes with the SW active.

## Tests

Vitest browser tests (OPFS source, IndexedDB store, launch consumer); Playwright `apps/web/e2e/offline.spec.ts`, `recent-files.spec.ts`, extended `privacy.spec.ts`; Lighthouse CI run against the preview (non-gating here; gating in C7c if chosen).

## Docs to update

`apps/web/README.md` (offline, storage, what is stored locally and how to clear it); tick "Service worker caching shell + WASM; web app manifest with `file_handlers`" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Stale SW caches** can pin users to old versions; the update flow and versioned caches must be tested across two deploys (preview A → preview B).
- **`file_handlers`** is Chromium-only; Safari/Firefox users won't get "Open with"; document it.
- **OPFS quotas** differ and Safari may evict; treat OPFS as a cache, never the only copy.
- **Recent remote URLs** are sensitive; default to remembering only local files? Owner to decide in review (recommendation: remember remote URLs only if the user opts in).

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
