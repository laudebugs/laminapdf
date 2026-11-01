# C2a — ADRs 0004, 0009, 0010, 0011: render backend, web client, platform adapter, Cloudflare and privacy

| Field | Value |
| --- | --- |
| ID | `C2a` |
| Status | `done` |
| Plan | [web reader §6 C2. Render backend and ADRs](../../plans/web-app.md#c2-render-backend-and-adrs) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C1a](./C1a-hayro-parity-spike.md), [C1b](./C1b-pdfium-wasm-spike.md), [X1](./X1-docs-link-checker.md) |
| Blocks | [C2b](./C2b-lamina-render-crate.md), [C3a](./C3a-viewer-worker-rpc.md), [C4a](../ready/C4a-ui-chrome-and-platform.md), [C4b](../ready/C4b-web-app-shell.md), [D2a](../ready/D2a-adr-0008-and-desktop-scaffold.md), [D5a](../ready/D5a-update-worker-and-adr-0014.md), [U1a](../ready/U1a-adr-0024-app-shell-ux.md) |
| ADRs | **Writes** 0004, 0009, 0010, 0011 (status `Accepted` on merge). Must stay consistent with [0001](../../decisions/0001-core-language-and-licence.md), [0002](../../decisions/0002-binding-strategy.md), [0003](../../decisions/0003-reader-tolerance-and-limits.md), [0005](../../decisions/0005-document-model.md), [0018](../../decisions/0018-test-strategy.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `docs/c2a-render-and-web-adrs` |
| PR title | `docs(decisions): add ADRs 0004, 0009, 0010 and 0011 for the web reader` |
| Size | M (four ADRs, ~400–600 lines of Markdown) |
| PR | [#105](https://github.com/laudebugs/lamina/pull/105) |

## Goal

Record the four Round C decisions before any product code depends on them, so C2b–C7 and the desktop rounds build on accepted ADRs instead of plan prose ([CLAUDE.md](../../../CLAUDE.md): "don't assume decisions that aren't in an ADR"). Each ADR turns the research recommendation plus the C1 spike numbers into a decision with consequences and rejected alternatives.

## Context

- [UI/UX plan §5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7) — ADR 0004's `RenderBackend` renders tiles `(page, scale, clip rect)` into bounded bitmaps and never allocates a full-page canvas (iOS Safari caps one canvas at ≈ 16.7 MP and all canvases at 224–384 MB); every renderer is read-only, all writes go through the save planner (ADR 0007). ADR 0024 ([U1a](../ready/U1a-adr-0024-app-shell-ux.md)) builds on 0009 and 0010.
- [Architecture plan §2 principles](../../plans/architecture.md#principles-each-becomes-part-of-an-adr) (4 rendering swappable, 5 UI framework off the hot path, 6 privacy first) and [§3 ADR table](../../plans/architecture.md#3-decisions-to-record-adrs).
- [Architecture §8 Q2](../../plans/architecture.md#8-open-questions-for-the-team): lamina **will** be a hosted service with login and cloud features, opt-in; the `auth` spike picks the login stack (owned by the editor/hosted todos, not here).
- [Research report](../../research/architecture/app-and-package-architecture.md): rendering, web client, and Cloudflare sections; "ADRs to record" list.
- Notes: [rendering](../../research/architecture/notes/rendering.md), [web-frontend](../../research/architecture/notes/web-frontend.md) (§1 framework, §5 workers and COOP/COEP, §6 OPFS/FS Access), [cloudflare](../../research/architecture/notes/cloudflare.md) (§1 static assets/previews/`cf` CLI, §4 privacy, auth, observability).
- Spike results: `spikes/hayro-parity/README.md` (C1a) and `spikes/pdfium-wasm/README.md` (C1b).
- [web-app.md §4 privacy and security](../../plans/web-app.md#privacy-and-security) and [§5 design summary](../../plans/web-app.md#5-design-summary).
- [docs/decisions/README.md](../../decisions/README.md) — ADR template and index.

## Scope

**In scope**

- `docs/decisions/0004-render-backend.md`
- `docs/decisions/0009-web-client-architecture.md`
- `docs/decisions/0010-platform-adapter.md`
- `docs/decisions/0011-cloudflare-topology-and-privacy.md`
- Index rows in `docs/decisions/README.md`, status updates in [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs), CLAUDE.md "accepted so far" sentence.

**Out of scope** (and which todo covers it)

- Code for the trait and adapters → [C2b](../done/C2b-lamina-render-crate.md), [C2c](../done/C2c-render-packages.md).
- The login/auth stack choice → the `auth` spike (editor/hosted todos); 0011 only fixes the principles and says the stack is added by amendment after that spike.
- ADR 0008 (desktop shell) and 0014 (distribution) → desktop todos.

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] **ADR 0004 — `RenderBackend`.** Decide and record:
  - [x] The trait surface: render a page rectangle at a scale (or target pixel size) into a caller-supplied RGBA buffer, with flags (annotations on/off, form widgets, print intent, background colour), plus page sizes/boxes and rotation; text, struct tree, annotation and link geometry come from lamina's model, **never** from the renderer.
  - [x] Adapters: PDFium (native via `pdfium-render` on desktop; PDFium-WASM in the web worker, wrapped by `@lamina/render-pdfium`) as default; hayro as second adapter from day one, diffed nightly; MuPDF prohibited (AGPL, ADR 0001); pdf.js not a backend (second document model).
  - [x] Pinned PDFium build: source, version, SHA-256 verification, update policy (security releases), as recommended by C1b.
  - [x] The switch-to-hayro criteria from C1a (pass-rate threshold on the pinned corpora, zero failures in named buckets, speed within X of PDFium) and who decides.
  - [x] Two-parsers rule: overlays (annotations, form values, page order) from lamina's model; content edits → incremental save → renderer reloads touched pages (C1b reload numbers).
  - [x] Packaging: `crates/laminapdf-render` (edge layer), `@lamina/render-pdfium` and `@lamina/render-hayro` optional so `@lamina/pdf` stays small.
- [x] **ADR 0009 — web client.** React 19 + React Compiler for chrome in `@lamina/ui`; framework-free `@lamina/viewer` (virtualiser, canvas pool, text layer, find, link layer) with no React import; one document worker per open document running `laminapdf-wasm` + the render adapter; hand-written typed RPC (request IDs, method union, priorities, transfer lists, `AbortSignal` cancellation, worker restart after a crash) instead of Comlink; rendering by transferred `ImageBitmap` onto pooled canvases (OffscreenCanvas as a later option); single-threaded WASM baseline, threaded variant only when `crossOriginIsolated` (record whether COOP/COEP ship in v0.1 — default: no); view state in an external store read with `useSyncExternalStore`; Vite 8, Vitest browser mode, Playwright on Chromium/Firefox/WebKit.
- [x] **ADR 0010 — Platform adapter.** The `Platform` interface (open file(s), save/download, print, clipboard write, open external URL, persistent storage, recent files) and capability flags (`canWriteInPlace`, `canWatchFiles`, `hasNativePrint`, `multiWindow`, `hasFileHandlers`, …); the web implementation's values; rule: UI code branches on capabilities, never on "is this Tauri"; how the desktop implementation (D-rounds) plugs in.
- [x] **ADR 0011 — Cloudflare topology, privacy and the hosted service.** One Worker `lamina-web` with static assets, `not_found_handling: "single-page-application"`, no `main` script and no `/api` routes in v0.1 (`run_worker_first: ["/api/*"]` reserved for later opt-in features); `_headers` (CSP `script-src 'self' 'wasm-unsafe-eval'`, no inline scripts, `Referrer-Policy: no-referrer`, minimal `Permissions-Policy`, HSTS, `X-Content-Type-Options`, `frame-ancestors`), immutable hashed assets; Wrangler pinned in CI (`cf` CLI tried later); preview URL per PR, staging from `main` and production only from `web-v*` release tags after the release-lane gates pass; remote documents via the `#url=` fragment only; **privacy rules**: no upload by default, no cookies, no third-party scripts, no fingerprinting, **no telemetry in v0.1** (any later telemetry optional, opt-in and off by default (owner decision 2026-10-29), cookieless, aggregate and content-free, error reports opt-in, and never containing document content, names or URLs); retention rules for future server features (opt-in per document, say what leaves the device, R2 lifecycle expiry); the **hosted service**: accounts, cloud storage and sync are opt-in features in later rounds, the app works fully without an account, the login stack is added by amendment after the `auth` spike; Workers Logs/observability settings so request URLs and headers carrying nothing sensitive is guaranteed by design.
- [x] Each ADR: `Status: Accepted`, date, Context, Decision, Consequences, Alternatives considered (Solid/Svelte, Comlink, OffscreenCanvas-in-worker, Pages, pdf.js, MuPDF, hayro-first, threaded-by-default), with links to research and spike results.
- [x] Update `docs/decisions/README.md`, [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs) statuses, and the CLAUDE.md "accepted so far" sentence.
- [x] Tick the four ADR to-dos under C2 in [web-app.md §9](../../plans/web-app.md#9-to-dos) and the Round C ADR line in [architecture §7](../../plans/architecture.md#7-to-dos).

## Acceptance criteria

- [x] ADR 0004 picks the default backend **per surface** (desktop, web, edge runtimes, later mobile) using the measured wasm sizes (PDFium about 2.15 MB gz; hayro measured in C1a or here) as well as fidelity ([shareable-libraries §8](../../plans/shareable-libraries.md#8-changes-to-existing-todos), [report: rendering](../../research/libraries/shareable-libraries.md#rendering-hayro-is-the-small-footprint-candidate-pdfium-wins-on-fidelity-at-about-2-mb))
- [x] Four ADR files exist with the template headings and status `Accepted`, each citing the research sections and, for 0004/0009, concrete numbers from the C1a/C1b READMEs.
- [x] 0004 states the trait surface precisely enough that C2b can implement it without further decisions, the pinned PDFium source and hash policy, and measurable hayro switch criteria.
- [x] 0009 lists the RPC requirements C3a must implement (IDs, priorities, transfer, cancellation, restart) and the threaded-variant policy.
- [x] 0010 lists every capability flag with web values.
- [x] 0011 states the v0.1 privacy guarantees as testable statements (they become C4c's Playwright assertions and C7c's privacy page) and the opt-in hosted-service rules.
- [x] Indexes and CLAUDE.md updated; `cargo xtask docs-links` (from [X1](../done/X1-docs-link-checker.md)) passes.

## Post-merge checks

None.

## Tests

Docs only. Run `cargo xtask docs-links` ([X1](../done/X1-docs-link-checker.md)) and `cargo xtask check` (Biome formats Markdown-adjacent JSON only; nothing else changes).

## Docs to update

`docs/decisions/README.md`, [architecture.md §3 and §7](../../plans/architecture.md#3-decisions-to-record-adrs), [web-app.md §9](../../plans/web-app.md#9-to-dos), `CLAUDE.md`.

## Risks and open questions

- **Domain and product name** ([web-app §8](../../plans/web-app.md#8-open-questions) Q1) are still open: write 0011 with `<domain>` placeholders and a note on how to fill them.
- **Telemetry** (Q3): decided by the owner 2026-10-29: none in v0.1, then optional and opt-in ([U5c](../ready/U5c-opt-in-usage-telemetry.md)); ADR 0011 records the allowed payload (aggregate, content-free) and retention.
- **Embedding/publishing `@lamina/viewer`** (Q4) affects how public the RPC types are; record "internal until the API settles" unless the owner decides otherwise.
- If C1b showed PDFium-WASM cannot meet the first-page budget, 0004 must say what changes (e.g. hayro for first paint) rather than paper over it.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome

- **[ADR 0004](../../decisions/0004-render-backend.md), the render backend:**
  - **Trait:** an object-safe `RenderBackend` / `RenderDocument` over the shared `ByteSource`, which renders tiles into straight RGBA with a `CancelToken` and `RenderLimits` (16.7 MP per request by default).
  - **Defaults per surface:** PDFium on every viewer surface; for library users and edge runtimes, no renderer is bundled and `@lamina/render-hayro` is optional.
  - **Pins:** `@embedpdf/pdfium` 2.15.1, whose `pdfium.wasm` SHA-256 is recorded, and `pdfium-binaries` `chromium/7881`, with an update policy.
  - **Rules:** C1a's switch criteria, the two-parsers rule (with C1b's reload numbers) and the packaging.
  - **hayro measured on wasm32 for this ADR:** 1.68 MB gzip with its built-in fonts and CMaps (79 % of PDFium's 2.14 MB) and 1.11 MB without them (52 %). These are upper bounds, without wasm-opt, and much less of a size win than the research assumed.
- **[ADR 0009](../../decisions/0009-web-client-architecture.md), the web client:**
  - **Packages:** React 19 chrome in `@lamina/ui`, and a framework-free `@lamina/viewer` with an external store.
  - **Workers:** one document worker per document, with range input by default and stepped saves.
  - **RPC requirements for C3a:** IDs, a typed union, priorities, transfer, cancellation, crash restart, long-work timeouts and the password rule.
  - **Pixels:** transferred `ImageBitmap`s onto pooled canvases.
  - **Threads:** single-threaded v0.1 with no COOP/COEP, and a threaded variant only when measured and cross-origin isolated.
- **[ADR 0010](../../decisions/0010-platform-adapter.md), the platform adapter:** the `Platform` and `PlatformFile` interfaces, and twelve capability flags with their web values and detection rules. UI code never checks the platform.
- **[ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md), Cloudflare and privacy:**
  - **Topology:** one static-assets Worker with no `main` in v0.1, per-PR previews, staging from `main`, and production only from `web-v*` tags.
  - **Documents:** they never pass through our servers (the `#url=` fragment, direct fetch).
  - **Guarantees and headers:** ten testable privacy guarantees for C4c and C7c, and the exact `_headers`.
  - **Later features:** content-free observability, plus the rules for telemetry, retention and the hosted service. The login stack is added by amendment after H1.

