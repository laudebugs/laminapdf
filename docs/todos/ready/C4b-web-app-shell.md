# C4b — `apps/web`: the reader on Cloudflare Workers static assets, with local file opening

| Field | Value |
| --- | --- |
| ID | `C4b` |
| Status | `ready` |
| Plan | [web reader §6 C4. App shell on Cloudflare](../../plans/web-app.md#c4-app-shell-on-cloudflare) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C2a](./C2a-render-adrs.md), [C4a](./C4a-ui-chrome-and-platform.md) |
| Blocks | [C4c](./C4c-remote-files-encryption-privacy.md), [C4d](./C4d-deploy-previews-and-production.md), [D7b](./D7b-settings-renderer-switch-and-download-page.md) |
| ADRs | Implements 0011 (Worker with static assets, `_headers`, CSP, no `/api`), 0009 (Vite 8, module worker), 0010 (web `Platform`) |
| Branch | `feat/c4b-web-app-shell` |
| PR title | `feat(web): add the web reader app on workers static assets with local file opening` |
| Size | L (600–1,500 lines incl. config and Playwright tests) |
| PR | — |

## Goal

Ship the first runnable reader: `apps/web`, a Vite 8 SPA that composes `@lamina/ui` and `@lamina/viewer`, served by one Cloudflare Worker with static assets and strict security headers. Users can open local PDFs by file input, drag and drop, paste and `showOpenFilePicker`, and see damaged-file notices. It runs locally with `wrangler dev` exactly as it will in production, and it has no server routes, so nothing can be uploaded.

## Context

- ADR 0011 (from [C2a](./C2a-render-adrs.md)) — topology, header values, Wrangler pinning.
- [Research: Cloudflare section](../../research/architecture/app-and-package-architecture.md#cloudflare-hosts-everything-for-about-5-a-month-with-heavy-jobs-in-containers) and [cloudflare notes §1](../../research/architecture/notes/cloudflare.md) — `not_found_handling: "single-page-application"`, no `main` script needed for a pure SPA, `_headers` applies to assets only, 25 MiB per asset (PDFium WASM fits), compat date ≥ 2025-04-01 navigation behaviour.
- [web-frontend notes](../../research/architecture/notes/web-frontend.md) §5 (COOP/COEP only for a threaded variant), §6 (`showOpenFilePicker` not Baseline; input + drag-drop fallback), §8 (Vite 8 + Rolldown, module workers + WASM).
- [web-app.md §4 functional 1, 9, 10](../../plans/web-app.md#functional) and [privacy and security](../../plans/web-app.md#privacy-and-security) (strict CSP values).

## Scope

**In scope**

- `apps/web/` (package name `@lamina/web`, `private: true`, never depended on): `package.json`, `vite.config.ts` (Vite 8, React plugin + React Compiler, worker `format: "es"`, WASM as hashed asset URLs), `index.html` (no inline script or style), `src/main.tsx`, `src/app.tsx`, `public/_headers`, `wrangler.jsonc`.
- `wrangler.jsonc`: `name: "lamina-web"`, `compatibility_date` pinned (≥ 2025-04-01), `assets: { directory: "./dist", not_found_handling: "single-page-application" }`, no `main`, no bindings; `preview_urls` set for C4d.
- `_headers`: CSP `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; style-src 'self'; img-src 'self' blob: data:; connect-src 'self' https:` (https: needed for `#url=` fetches in C4c — confirm in ADR 0011; this is the production policy, and C4c adds a test-mode build variant that also allows `http://localhost:*`/`http://127.0.0.1:*` for local fixture hosts, never in production); `object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'`; `Referrer-Policy: no-referrer`; `Permissions-Policy` denying camera, microphone, geolocation, etc.; `X-Content-Type-Options: nosniff`; `Strict-Transport-Security`; `Cross-Origin-Opener-Policy: same-origin` (no COEP in the single-threaded baseline, per ADR 0009/0011); all of the above on a `/*` rule. **Caching** (checked against [Cloudflare's `_headers` docs](https://developers.cloudflare.com/workers/static-assets/headers/): rules match the request path, not the served file, so a rule keyed on `index.html` never matches `/` or SPA-fallback deep links; every matching rule's headers are inherited and a header set twice is comma-joined; `! Header` detaches one): `/assets/*` gets `Cache-Control: public, max-age=31536000, immutable`, and **no** `/*` rule sets `Cache-Control` (that would join into `no-cache, public, max-age=31536000, immutable` on assets). HTML — `/`, `/index.html` and every SPA-fallback path — keeps the static-assets default `Cache-Control: public, max-age=0, must-revalidate` (revalidated on every load via the ETag), so users are never pinned to a stale shell. If a later `/*` rule ever needs a `Cache-Control`, the `/assets/*` rule must first detach it with `! Cache-Control`.
- Opening flows: file input, drag-and-drop on the whole window, paste of a PDF file from the clipboard, `showOpenFilePicker` where available — all via `Platform.openFiles` → `ByteSource` over `File.slice()` → viewer worker.
- PDFium and lamina WASM lazy-loaded at app start in parallel with file selection (from C3a), served as hashed static assets.
- Repair/warning notices and error states wired from viewer events to `@lamina/ui`.
- Debug menu (`?debug`) switching backend PDFium ↔ hayro.
- Playwright e2e project for `apps/web` against `wrangler dev` (production-equivalent assets + headers).

**Out of scope** (and which todo covers it)

- `#url=` remote files, range requests, encrypted-file prompt flow, privacy assertions → [C4c](./C4c-remote-files-encryption-privacy.md).
- CI deploys, preview URLs, production → [C4d](./C4d-deploy-previews-and-production.md).
- Service worker, manifest, `file_handlers` → [C7b](./C7b-offline-pwa.md).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Scaffold `apps/web` (`"name": "@lamina/web"`) with Vite 8; add to `pnpm-workspace.yaml` (already covers `apps/*`); depend on `@lamina/ui`, `@lamina/viewer`, `@lamina/render-pdfium`, `@lamina/render-hayro` via `workspace:*`.
- [ ] Pin Wrangler as a devDependency (exact version) and add scripts: `dev` (Vite), `build`, `preview` (`wrangler dev` serving `dist/`), `e2e`.
- [ ] `wrangler.jsonc` and `public/_headers` as in scope; verify headers with `curl -I` against `wrangler dev` for `/`, `/index.html`, `/some/deep/link` (SPA fallback 200), `/assets/*.js`, `/assets/*.wasm` (`Content-Type: application/wasm`).
- [ ] Ensure no inline scripts/styles in the build (Vite `build.modulePreload.polyfill: false` if it injects inline code; check `index.html` output in a test).
- [ ] App composition: `PlatformProvider(createWebPlatform())` → `ReaderShell` with viewer slot; reader controller connects `openFiles` → viewer worker open; title updates to the document title (from metadata) without sending anything anywhere.
- [ ] Drag-and-drop: overlay on dragenter, accept the first PDF (`application/pdf` or `%PDF-` sniff), reject others with a message; paste handler for `ClipboardEvent.clipboardData.files`.
- [ ] Error and repair notices: warnings from the worker (ADR 0003 codes) → `RepairNotice`; fatal open errors → `ErrorBanner` with "Try another file".
- [ ] Debug menu: backend switch reopens the document with the other backend; shows render timings.
- [ ] Playwright e2e (`apps/web/e2e/`): open via input; drag-drop (synthetic `DataTransfer`); paste; `showOpenFilePicker` path mocked in Chromium; damaged file shows "repaired" notice; non-PDF rejected; deep link returns the app (SPA fallback); response headers on `/` and assets match `_headers`; **CSP violations = 0** (listen to `securitypolicyviolation`); no request to any origin other than self (route interception) during local-file flows.
- [ ] README for `apps/web` (run, build, preview with wrangler, header policy summary linking ADR 0011).

## Acceptance criteria

- [ ] `pnpm --filter @lamina/web build && pnpm --filter @lamina/web preview` (filter by package name; a bare `apps/web` is a name selector that matches nothing and exits 0) serves the reader locally through Wrangler; opening `fixtures/corpus/hello.pdf` via input, drag-drop and paste renders page 1 in Chromium, Firefox and WebKit (Playwright).
- [ ] Response headers on `/`, `/index.html`, a deep link and a hashed asset match the ADR 0011 policy (asserted in e2e), with **exact** `Cache-Control` values: `public, max-age=0, must-revalidate` (exactly one value, no `immutable`) on `/`, `/index.html` and the deep link; `public, max-age=31536000, immutable` (exactly, no comma-joined extras) on `/assets/*.js` and `/assets/*.wasm`.
- [ ] No inline script/style in `dist/index.html`; zero CSP violations during the e2e suite.
- [ ] Opening a local file makes no network request except for same-origin static assets (asserted by request interception).
- [ ] `hello-broken.pdf` opens with the repaired notice; a `.txt` file is rejected with a message.
- [ ] `cargo xtask check ts` includes the app's typecheck, Biome and unit tests; e2e runs in the full lane.

## Post-merge checks

None.

## Tests

Playwright e2e in `apps/web/e2e/*.spec.ts` (three engines) against `wrangler dev`; small Vitest unit tests for the PDF sniffing and drop handling in `apps/web/src/**/*.test.ts`.

## Docs to update

`apps/web/README.md`; `CLAUDE.md` (layout `apps/web`, commands to run the app); tick "Create `apps/web` (Vite 8) with `wrangler.jsonc`, SPA fallback, `_headers`, CSP" and the local-open part of "Implement open via input, drag-drop, paste, `showOpenFilePicker`…" in [web-app.md §9](../../plans/web-app.md#9-to-dos) (the `#url=` part is ticked by C4c).

## Risks and open questions

- **`wasm-unsafe-eval` support** across target browsers (Safari ≥ 16 supports it); verify the PDFium glue needs nothing beyond it (no `eval`/`new Function`).
- **COOP `same-origin`** is harmless without COEP but blocks `window.opener` for future OAuth popups; ADR 0011 should note that the hosted-service login flow (later) must use redirects or a separate origin.
- **`_headers` don't apply to Worker-generated responses**; v0.1 has no Worker code, so all responses are assets — keep it that way (an e2e checks there is no `main`).
- **Vite 8 module worker + WASM URL handling** — verify the built worker resolves `.wasm` URLs relative to the asset base.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
