# T5b — Reopen each document where the reader left it

| Field | Value |
| --- | --- |
| ID | `T5b` |
| Status | `ready` |
| Plan | [trust and proof §T5](../../plans/trust-and-proof.md#t5-long-reading) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [C6b](../done/C6b-links-and-history.md), [C7b](../ongoing/C7b-offline-pwa.md) |
| ADRs | Amends [0010](../../decisions/0010-platform-adapter.md) §1 (a `positions` member, an option on `recent.reopen`) and [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) §3 guarantee 9 (what the app stores); follows [0024](../../decisions/0024-app-shell-ux.md) §3 (a command with a visible home) and [0009](../../decisions/0009-web-client-architecture.md) (the fingerprint is computed in the document worker) |
| Branch | `feat/t5b-reopen-at-last-position` |
| PR title | `feat(web): reopen each document where the reader left it` |
| Size | L (≈ 450–650 lines incl. tests) |
| PR | — |

## Goal

SumatraPDF is the most-loved reader of the decade on Hacker News, and its saved position is one of the things users name: "when you re-open a PDF, opens it at the position you were last time(!!!)" ([14839643](https://news.ycombinator.com/item?id=14839643)). 47 comments ask for it, and readers that forget complain about it: Apple's Books ([25448417](https://news.ycombinator.com/item?id=25448417)) and Okular, which "can’t remember opened tabs" ([37994382](https://news.ycombinator.com/item?id=37994382)). This PR remembers, on the device only, each document's page, place on the page, zoom and view rotation, under a fingerprint of the file rather than its name. Each document reopens there, and the document that was open when the tab or browser closed comes back where the browser allows. Nothing about a document leaves the device (ADR 0011), and one command forgets every position.

## Context

- **Evidence:** the [readers note](../../research/hacker-news/notes/readers.md) (want 18, "Reopen at the last page": 47 comments, 9 in 10 on topic in the spot-check; want 8, sync across devices, is out of scope here) and the report's [R7](../../research/hacker-news/pdf-users-on-hacker-news.md#r7-serve-long-reading-reflow-a-dark-page-papers-and-manuals).
- **[ADR 0010](../../decisions/0010-platform-adapter.md):** `Platform.storage`, `Platform.recent`, `canReopenRecent`, `hasOpfs`, and rule 5 (a new member goes into the ADR's tables in the same PR). **[ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md)** §2 (a remote document's URL is never stored) and §3 guarantee 9, which lists everything the app stores.
- **[C4a](../done/C4a-ui-chrome-and-platform.md)** built the web platform: `createWebPlatform()` in `packages/ui/src/platform/web.ts`, the `lamina-pdf` IndexedDB database and its `kv` store (`packages/ui/src/platform/idb.ts`, with a memory store where IndexedDB doesn't open), and the recent list (20 entries keyed by name and size, file handles beside them where `canReopenRecent`).
- **[C7b](./C7b-offline-pwa.md)** adds the database's schema version and migrations, OPFS copies of opened files, the recent rows on the home screen (with a last page), and the "Storage and privacy" settings section in C7a's dialog ("Clear recent files and cached documents", "Remember recent local files", "Remember remote URLs", off by default).
- **[C6b](../done/C6b-links-and-history.md):** `NavigationHistory`, whose entries are `{ page, scroll offset, zoom }`, and the viewer call that restores one. A remembered position is the same kind of value. **[C6a](../done/C6a-thumbnails-outline-page-labels.md):** the `#page=` and `#nameddest=` deep links, read through C4c's `parseFragment`.
- **The viewer:** `Viewer.pointAt(x, y)` and `positionOf(point)` with `DocumentPoint` (a page and fractions of its box), `getState()`/`subscribe`, and the worker's `openDocument`, which returns `OpenedDocument` (`packages/viewer/src/protocol.ts`, `src/worker/document.ts`).
- **Design (Paper):** [R01 Home / open](../../design/paper/reader-a.md#r01-home--open) (recent rows: "This device · pages · last page"; "Choose the file again to reopen"), [R15](../../design/paper/reader-b.md#r15-offline-install-and-update) ("Reload to use it; you'll come back to page 3"). Desktop: [S01](../../design/paper/desktop-shell.md#s01-start-window) and [S02](../../design/paper/desktop-shell.md#s02-window-with-document-tabs-and-session-restore), built by [D3b](./D3b-recent-files-and-file-watching.md) (recent entries with page and zoom, by path) and [D4a](./D4a-windows-tabs-and-session-restore.md) (session restore with each tab's own view).
- **Spec:** ISO 32000-2 §14.4 (file identifiers: `/ID[0]` is meant to stay the same through incremental updates); §7.5.6 (incremental updates append to the file).

## Scope

**In scope**

- **The fingerprint (decided).** `documentFingerprint(bytes)` in `packages/viewer/src/worker/fingerprint.ts` returns `"fp1:" + hex(SHA-256(…))`, computed with `crypto.subtle.digest` in the document worker over:
  - the ASCII tag `laminapdf.fp/1` and a zero byte;
  - then the file's head: from offset 0 through the end of the first `%%EOF` marker that lies within the first 64 KiB, or the first 64 KiB when no marker lies there (the whole file when it is smaller).
  An incremental update appends to the file (§7.5.6), so it leaves the head alone, and a file reopens at its position after someone adds an annotation or a signature. The first 64 KiB differ between documents, including documents whose generator writes the same `/ID` in every file. `/ID[0]` stays out of the hash: a file without one gains one on its first save (Lamina PDF's writer derives it; compare `fixtures/golden/hello.pdf` with `fixtures/golden/incremental-set-info.pdf`), which would change the key. A full rewrite changes the head, so its output starts without a position. `OpenedDocument` gains `fingerprint: string`; the worker already holds the bytes in v0.1, so computing it reads nothing new.
- **The store.** `Platform.positions: ReadingPositions` (new, in `packages/ui/src/platform/types.ts`), with `get(fingerprint)`, `set(fingerprint, entry)`, `remove(fingerprint)` and `clear()`. An entry is `{ view, pageCount, readAt }`:
  - `view` is opaque to `@laminapdf/ui` (a structured-clone value that the app checks on read), so the UI package keeps no copy of the viewer's `Zoom` and `DocumentPoint` types;
  - the web implementation (`packages/ui/src/platform/positions.ts`) is an IndexedDB object store `positions` in the `lamina-pdf` database (C7b's schema version plus one, with an index on `readAt`). Each `set` trims the store to **500 entries**, the oldest `readAt` first, in the same transaction, so two tabs can't race past the cap. Where IndexedDB doesn't open, it falls back to memory, as `idb.ts` does;
  - an entry holds no name, path, URL, bytes or document text.
- **Recording.** `apps/web/src/reading/positions.ts` (new) saves the view 1 s after the viewer's state stops changing, when the page becomes hidden (`visibilitychange`) and when the document closes. The view is the type C6b's `NavigationHistory` stores; if C6b's entry doesn't carry the in-page offset, this PR adds `Viewer.viewPosition()` and `Viewer.restore(position)` to `@laminapdf/viewer`, with the position `{ point: DocumentPoint, zoom, rotation }`, where `point` is the document point at the middle of the viewport's top edge (`pointAt(width / 2, 0)`), and C6b's history uses it too: one type for both.
- **Restoring on open**, by whatever way the file came in (picker, drop, paste, the recent list, a file handler, `#url=`):
  - a `page=` or `nameddest=` key in the fragment wins, and nothing is restored;
  - otherwise the app reads the entry once the worker returns the fingerprint, and restores it before the first paint: `viewer.open(document)`, then `viewer.restore(view)` in the same task, the page clamped to the page count, zoom and rotation applied;
  - the start of the document becomes a `NavigationHistory` entry, so `nav.back` reads "Back to page 1" and goes there;
  - the live region (C7a's `Announcer`) says "Opened at page 12, where you stopped reading".
- **The document open when the app last closed.** A `session` key in `Platform.storage` holds `{ recentId }`. It is written when a local document that is in the recent list opens (so only while "Remember recent local files" is on), and removed when that document closes or the home screen shows. On start, with no `url` in the fragment:
  - where `canReopenRecent` and the browser kept the permission, the document reopens without a prompt (`recent.reopen(id, { ask: false })`, new: it resolves `undefined` instead of calling `requestPermission`, which needs a click), then its position comes back;
  - else, when C7b holds an OPFS copy of it, it reopens from the copy;
  - else the home screen lists it first as **Continue reading** with "page 12 of 40". Activating it reopens through the handle (the click allows the prompt) or the picker ("Choose the file again to reopen"), and the fingerprint brings the position back.
  A `#url=` document comes back with the browser's own tab restore, which keeps the fragment; its position comes back by fingerprint, and no URL is stored (ADR 0011 §2). Only C7b's opt-in "Remember remote URLs" lets the recent list reopen one.
- **The recent rows read positions.** C7b's recent entries gain the `fingerprint`, and their "last page" comes from `positions`. If C7b stored a `lastPage` of its own, the migration moves it into `positions` and drops the field: one home for a position.
- **Privacy controls.**
  - "Remember where I stopped reading" in C7b's "Storage and privacy" section (preference key `positions.enabled`, default on, "Kept in this browser only"). Turning it off clears the store and the session record and stops writing.
  - The command `file.clear-reading-positions` ("Forget reading positions", group `file`, in U1b's registry): its visible home is a button in that section, and the palette lists it too. It clears the store and the session record.
  - C7b's "Clear recent files and cached documents" clears positions too, since it empties the database.
- **ADR amendments**, in this PR: ADR 0010 §1 gets `positions` and its web value (the IndexedDB store) and `recent.reopen`'s `{ ask }` option; ADR 0011 §3 guarantee 9 adds "reading positions: a fingerprint of each document (a hash of its first bytes) with its page, place on the page, zoom, rotation and when it was read; never its name, bytes or URL" and "the recent-list ID of the document open now".
- **Desktop.** `createDesktopPlatform()` may reuse the web implementation: the webview's IndexedDB lives in the app's data folder. D3b's and D4a's own per-path and per-tab views stay; see Docs to update.

**Out of scope** (and which todo covers it)

- Positions that follow the reader across devices (want 8, 159 comments): it needs accounts and ADR 0011 §6's end-to-end encryption; the hosted-service rounds ([H4](./H4-document-sync.md)).
- Desktop tabs and windows after a restart: [D4a](./D4a-windows-tabs-and-session-restore.md). The desktop's recent list: [D3b](./D3b-recent-files-and-file-watching.md).
- A position inside reader mode's reflowed text ([U4b](./U4b-reader-mode-from-structure-tree.md), [U4c](./U4c-approximate-reader-mode.md)): later, once reader mode exists.
- Page appearance remembered per document (R10's "for this document only"): [U2c](./U2c-page-appearance-modes.md). It can key its records by the same fingerprint.

## Tasks

- [ ] Claim via the work PR (see Definition of done); a fresh worktree off `main`. Read C6b's and C7b's merged code first: names below follow their todos.
- [ ] `packages/viewer/src/worker/fingerprint.ts` and `OpenedDocument.fingerprint` (`protocol.ts`, `worker/document.ts`); a restarted worker reports the same value.
- [ ] `Viewer.viewPosition()`/`restore(position)`, or C6b's equivalent, with a test that a restore before the first frame renders no other page first.
- [ ] `ReadingPositions` in `packages/ui/src/platform/types.ts`; the IndexedDB store, the cap and the memory fallback in `positions.ts`; `createWebPlatform()` wiring; C7b's migration with the new store and the recent entries' `fingerprint`; `recent.reopen(id, { ask })`.
- [ ] `apps/web/src/reading/positions.ts`: recording, restoring, the history entry, the announcement, the deep-link rule.
- [ ] The session record and the home screen's **Continue reading** row (R01's recent-row component, with a "page 12 of 40" line).
- [ ] The setting, the command and their messages; the ADR 0010 and ADR 0011 amendments; the docs below.

## Acceptance criteria

- [ ] **The fingerprint** (Vitest, Node): the same bytes give the same value; `fixtures/golden/hello.pdf` and `fixtures/golden/incremental-set-info.pdf` (the same file plus an incremental update that adds an `/ID`) give the same value; `fixtures/golden/full-rewrite-hello.pdf` and two different corpus files give different ones; a file smaller than 64 KiB and one larger both work.
- [ ] **Reopen** (Playwright, Chromium, Firefox, WebKit, `apps/web/e2e/reading-position.spec.ts`): open a 40-page fixture built in the test with `@laminapdf/sdk`'s `Builder`, go to page 12 at 150 % with a scroll offset, close it, and open the same file again. It shows page 12 with the remembered point within 16 CSS px of the viewport's top, at 150 %. The client's `render-start` events show no render of page 1 before the restore. `nav.back` reads "Back to page 1" and goes there.
- [ ] **Keyed by content:** a renamed copy of the file restores; a different file with the same name and size doesn't; the file after an incremental save made in the test with `@laminapdf/sdk` (`apply` a `SetInfo`, then `save`) restores; `#page=3` wins over the stored position.
- [ ] **After a restart:** with a stored handle whose `queryPermission` answers `granted` (a fake handle in Vitest browser mode, since Playwright can't drive the file picker), start-up reopens the document at its position and never calls `requestPermission`. Without a handle (Firefox and WebKit in Playwright), the home screen shows **Continue reading** with the page, and picking the file restores it. A `#url=` document reloaded with its fragment returns to its position.
- [ ] **Privacy:** the test reads every value in the `positions` store and finds only `view`, `pageCount` and `readAt` under fingerprint keys. With "Remember where I stopped reading" off, the store is empty and nothing is written. `file.clear-reading-positions` empties it from the settings section and from the palette. The privacy suite (C4c's `privacy.spec.ts`) finds no request carrying the fingerprint, and the network indicator ([U4a](../done/U4a-network-indicator-and-privacy-proof.md)) stays at 0 while a local file reopens.
- [ ] **The cap:** writing a 501st entry removes the oldest one (Vitest browser mode, real IndexedDB), and two pages writing at once leave at most 500.
- [ ] **Speed:** the perf harness's "first page, warm" figure ([C3c](../done/C3c-viewer-perf-harness.md)) with a stored position on page 12 is within 10 % of the figure for a first open (recorded in the PR).

## Post-merge checks

- [ ] In Chrome, on the staging deploy: open a local file from the picker, choose "Allow on every visit" when Chrome offers it, read to page 12, close the browser and start it again; the reader reopens the file at page 12 without a prompt (recorded in the done todo).

## Tests

- Vitest (Node): `packages/viewer/test/fingerprint.test.ts` over the golden and corpus fixtures.
- Vitest browser mode: `packages/ui/test/browser/positions.browser.test.ts` (the store, the cap, `clear`, the memory fallback, the `{ ask: false }` reopen with a fake handle) and `packages/viewer/test/browser/` for `restore`.
- Playwright: `apps/web/e2e/reading-position.spec.ts` (three engines) and the privacy suite's new case.

## Docs to update

- ADR 0010 (§1 interface and §2 table; an amendment line in its header) and ADR 0011 (§3 guarantee 9; an amendment line in its header).
- [R01](../../design/paper/reader-a.md#r01-home--open): the **Continue reading** row, recorded as a deviation ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)). The design language's [command appendix](../../design/paper/design-language.md#appendix-command-ids): `file.clear-reading-positions`, and `positions.enabled` in its list of preference keys.
- `apps/web/README.md` (what the reader stores on the device and how to clear it) and `packages/ui/README.md` (`Platform.positions`).
- [D3b](./D3b-recent-files-and-file-watching.md) and [D4a](./D4a-windows-tabs-and-session-restore.md), if still in `ready/`: D3b's recent entries store the fingerprint and read page and zoom from `Platform.positions` instead of their own fields; D4a's tabs keep their own view, and a document opened outside a restored tab uses `Platform.positions`.
- [Trust and proof §11](../../plans/trust-and-proof.md#11-to-dos): tick T5b.

## Risks and open questions

- **Collisions.** Two files with the same first 64 KiB (or the same first revision up to `%%EOF`) share a position. The cost is a wrong page on open, clamped to the page count; no data is lost.
- **Safari's storage eviction.** Safari deletes a site's script-writable storage after seven days without a visit unless the web app is installed, so positions can vanish there. Say so in `apps/web/README.md`; nothing in this PR can prevent it.
- **Writes on close.** IndexedDB writes during `pagehide` may not finish; the 1 s debounce keeps the stored position at most a second behind.
- **Open question for the owner: remote documents.** The recommendation is to remember `#url=` documents' positions by default, because only the fingerprint is stored, never the URL. The alternative is to follow C7b's "Remember remote URLs", off by default, which would leave shared links without this feature.
- **Open question for the owner: the cap.** The recommendation is 500 documents, about 75 KB in IndexedDB; the oldest position goes first.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
