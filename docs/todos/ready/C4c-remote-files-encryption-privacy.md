# C4c — Remote files via `#url=`, encrypted files, and the privacy test suite

| Field | Value |
| --- | --- |
| ID | `C4c` |
| Status | `ready` |
| Plan | [web reader §6 C4. App shell on Cloudflare](../../plans/web-app.md#c4-app-shell-on-cloudflare) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B2b](./B2b-standard-security-decryption.md), [B6b](./B6b-lamina-pdf-package.md), [C4b](./C4b-web-app-shell.md) |
| Blocks | [C5a](./C5a-text-layer-and-selection.md), [C6a](./C6a-thumbnails-outline-page-labels.md), [C6c](./C6c-print-download-properties.md), [C7b](./C7b-offline-pwa.md), [D2b](./D2b-engine-process.md) |
| Dependency notes | [C4b](./C4b-web-app-shell.md); library B2 (standard security handler decryption and encrypted fixtures; [pdf-library B2](../../plans/pdf-library.md#b2-document-model)), exposed through **B6** |
| ADRs | Implements 0011's privacy guarantees (fragment URLs, no upload, no referrer) as tests; follows [0005](../../decisions/0005-document-model.md) §2 (`ByteSource`), [0003](../../decisions/0003-reader-tolerance-and-limits.md) (wrong-password and repair codes) |
| Branch | `feat/c4c-remote-and-encrypted-files` |
| PR title | `feat(web): open remote pdfs from the url fragment and encrypted pdfs, with privacy tests` |
| Size | L (600–1,500 lines incl. tests) |
| PR | — |

## Goal

Complete the reader's opening story and prove its privacy promise. Remote PDFs open from `https://<domain>/#url=<encoded URL>` — a fragment, so browsers never send it to our server — fetched **directly** from their host with HTTP range requests through a `ByteSource`. Password-protected PDFs prompt for a password that stays in the worker. And a Playwright privacy suite asserts, on every PR, that no request body carries document bytes and no request to our origin (URL, request line or headers, including `Referer`) contains the remote document URL.

## Context

- [web-app.md §4 functional 1](../../plans/web-app.md#functional) and [privacy and security](../../plans/web-app.md#privacy-and-security) — exact requirements; [§3 users](../../plans/web-app.md#3-users-and-use-cases) (`https://<domain>/#url=…` links).
- ADR 0011 (from [C2a](./C2a-render-adrs.md)) — testable privacy statements; no CORS proxy in v0.1 ([§8 Q2](../../plans/web-app.md#8-open-questions)).
- [04 §7 encryption](../../research/pdf/04-interactive-security.md#7-encryption-76) — standard security handler; user vs owner passwords.
- [pdf-library B2](../../plans/pdf-library.md#b2-document-model) — `OpenOptions` password, wrong-password error code, encrypted fixtures for every revision.
- [C3a](./C3a-viewer-worker-rpc.md) — `ByteSource` interface and the password-once rule.

## Scope

**In scope**

- `packages/viewer/src/source/http-range.ts`: `HttpRangeByteSource` (HEAD or `Range: bytes=0-0` probe for size and `Accept-Ranges`, ranged `fetch` with `AbortSignal`, block cache (e.g. 64 KiB blocks, LRU, byte budget), readahead, fallback to full download when ranges unsupported, progress events, `credentials: "omit"`, `referrerPolicy: "no-referrer"`, `cache: "no-store"` option).
- `apps/web/src/open/url-fragment.ts`: parse `location.hash` (`#url=` encoded; also `#page=N` deep links; ignore unknown keys), validate scheme (`https:` only; `http:` refused with explanation except `localhost` in dev), react to `hashchange`.
- CORS failure detection → explanation screen ("This site doesn't allow other sites to read the file. Download it and open it here."), with a link to the URL (opened with `rel="noreferrer noopener"`); no proxy.
- Encrypted files: worker returns `needs-password`; UI shows `PasswordDialog`; the password is sent once to the worker and passed to both `Document::open` (lamina) and the PDFium adapter; wrong password → retry; owner-password-only files open without prompt (permissions noted, not enforced beyond display in v0.1 — record in properties later).
- Privacy suite `apps/web/e2e/privacy.spec.ts`.

**Out of scope** (and which todo covers it)

- OPFS caching of large remote/local files and recent files → [C7b](./C7b-offline-pwa.md).
- Opt-in CORS proxy (open question; would be a later server feature under ADR 0011).
- Text extraction from encrypted files → [C5a](./C5a-text-layer-and-selection.md) (acceptance there).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`.
- [ ] Implement `HttpRangeByteSource` with unit tests against a local test server (Playwright's `route` or a tiny static server in the test harness) that supports/doesn't support ranges, returns wrong `Content-Range`, changes `ETag` mid-read (→ error `source-changed`), or is slow (abort works).
- [ ] Hook it into the worker: the worker (not the main thread) performs the fetches, so bytes never cross into the main thread; ADR 0011's CSP `connect-src` allows `https:`.
- [ ] Fragment parsing and validation with unit tests (encoded/unencoded URLs, `#url=` plus `#page=`, malicious inputs like `javascript:`/`data:`/`file:` rejected).
- [ ] After reading the fragment, **do not** put the URL into any `history.pushState` path or query; keep it in the fragment only; no `document.title` containing it.
- [ ] CORS/network error classification (`TypeError` from fetch with opaque failure → `cors-or-network`), explanation UI, retry.
- [ ] Encrypted flow end to end, including cancel and retry; `PasswordDialog` from C4a; worker zeroes its copy of the password string reference after open where possible (document the JS limits on zeroing).
- [ ] Privacy suite (`privacy.spec.ts`), run against `wrangler dev` with an extra local "remote host" serving fixtures with CORS + ranges:
  - [ ] Record every request (`page.on("request")` + service-worker requests + worker requests via `context.on("request")`).
  - [ ] **No request body contains document bytes**: for each request with a body, assert it does not contain any 32-byte window of the opened file (rolling-hash check) nor `%PDF-`.
  - [ ] **No request to our origin contains the remote document URL** (or its encoded forms, or its host) in URL, request line, any header including `Referer`; assert `Referer` is absent on cross-origin fetches.
  - [ ] Requests to our origin are only `GET` for static assets (no POST/PUT, no `/api`).
  - [ ] No cookies set or sent (`context.cookies()` empty; no `Set-Cookie` in responses).
  - [ ] No third-party origins contacted except the remote document host chosen by the user.
  - [ ] Same checks for local-file open and encrypted-file open (password string never appears in any request).
- [ ] Playwright functional tests: `#url=` with CORS + ranges opens and fetches only needed ranges for page 1 of a large fixture (assert total bytes fetched < file size); no-range server falls back to full download; no-CORS server shows the explanation; encrypted fixtures (every B2 revision: R2–R6, user/owner/empty passwords) open after the prompt; wrong password shows an error.

## Acceptance criteria

- [ ] `https://localhost:<port>/#url=<cors-host>/big.pdf` renders page 1 after fetching less than 25 % of a 50 MB fixture (range requests), in Chromium, Firefox and WebKit.
- [ ] A remote file without CORS shows the explanation, never silently proxies.
- [ ] All B2 encrypted fixtures open after the password prompt; wrong passwords produce the wrong-password message; `enc.pdf` from `fixtures/corpus` opens.
- [ ] The privacy suite passes in all three engines and runs in CI on every PR (full lane), covering local, remote and encrypted opens.
- [ ] The remote URL appears nowhere except in the fragment and in requests to the remote host itself.

## Tests

Vitest unit tests for `HttpRangeByteSource` and fragment parsing; Playwright `apps/web/e2e/remote.spec.ts`, `encrypted.spec.ts`, `privacy.spec.ts` with a local fixture host (CORS on/off, ranges on/off). Large remote fixtures are generated at setup, not committed.

## Docs to update

`apps/web/README.md` (`#url=` usage, CORS requirement for hosts, privacy guarantees and how they're tested); tick the `#url=` part of the open to-do, "Add password prompt…" and "Playwright test asserting no request body contains document bytes…" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Range requests and compression**: servers that gzip responses ignore ranges or return mismatched lengths; detect via `Content-Range` and fall back.
- **Changing remote files** mid-session (ETag change) must not produce mixed-revision rendering; fail clearly.
- **Very large remote files** without range support must not exhaust memory: stream to OPFS when C7b lands; for now cap and show a message above a size threshold.
- **Password zeroing** in JS isn't guaranteed (strings are immutable); document the residual risk and keep the password only in the worker.
- Detecting CORS vs offline vs DNS is imperfect (`fetch` gives an opaque `TypeError`); phrase the message carefully.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
