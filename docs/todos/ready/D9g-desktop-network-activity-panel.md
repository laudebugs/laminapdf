# D9g — Desktop: the network activity panel

| Field | Value |
| --- | --- |
| ID | `D9g` |
| Status | `ready` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | None |
| ADRs | Follows [ADR 0014](../../decisions/0014-desktop-distribution.md) (updates, crash reports, the store build's "no request" rule) and [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the privacy page) |
| Branch | `feat/d9g-desktop-network-panel` |
| PR title | `feat(desktop): the network activity panel` |
| Size | M (200–400: the panel is shared; the main process's request log is new) |
| PR | — |

## Goal

Make Privacy ▸ Network activity open a panel that shows what the app has sent over the network, so the promise that the reader works offline can be checked by the person using it. The item is disabled in the desktop today.

## Context

- [#793](https://github.com/laudebugs/laminapdf/issues/793) and its root-cause comment: `privacy.network-activity` is `delegated()`; the web handles it in `apps/web/src/app.tsx` (about line 267) by opening the inspector's `NETWORK_PANEL`.
- `apps/web/src/network/` (`monitor.ts`, `classify.ts`, `panel.tsx`, `report.ts`, `page-feed.ts`): a monitor over the page's requests, a classifier and the panel. Check how much is web-specific; the monitor and panel should move to a shared package if the desktop uses them (one home per concept).
- **What the desktop can show.** The webview's own requests are only its app's pages and tiles over the app's own schemes. The requests that leave the machine come from the main process: the update check (`apps/desktop/src-tauri/src/update`) and crash report uploads (`crash`). The webview cannot see them, so the main process needs a small request log that sends each outgoing request to the webview (host, purpose, time, size, no URL query or file name). Check [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) and the privacy page for the wording the panel must agree with.
- **The store build.** [ADR 0014 §11](../../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build): the Mac App Store build's own code makes no request at all. Its panel therefore shows an empty log with a plain statement: "This app has made no network request." The panel must not imply a request when there was none.
- The direct build lists its update checks (when updates are on) and crash report sends (when the person opted in), each labelled, and says when updates and reports are off.

## Scope

**In scope**

- A request log in the main process that records each outgoing request of the updater and the crash reporter (kind, host, time, outcome) and nothing else, held in memory for the session.
- An event or command that gives the log to the webview, and the panel showing it, with the web's wording and layout where they apply.
- `privacy.network-activity` handler in the desktop, enabled with or without a document (check the command's `enabled` rule).
- The store build's empty state, and the direct build's states for updates and reports being off.

**Out of scope**

- Blocking or approving requests; a firewall.
- A persistent log on disk (it would hold the user's history).

## Tasks

- [ ] Fresh worktree; claim in the work PR.
- [ ] Read the web's network folder and decide what moves into a shared package.
- [ ] Add the log to the update and crash code paths, with a test that nothing else (no path, no query, no file name) is stored.
- [ ] Wire the panel and the handler in `apps/desktop/src/app.tsx`; make the menu item available.
- [ ] Tests, browser check, docs.

## Acceptance criteria

- [ ] In the real app on macOS, Windows and Linux, Privacy ▸ Network activity is enabled and opens the panel.
- [ ] In the direct build, a check for updates adds an entry that names the host and purpose; with updates off and crash reports off the panel says nothing has been sent and why.
- [ ] In the store build the panel says the app has made no network request, and the log is empty after the app has run, opened a file and been left idle for a minute.
- [ ] No log entry holds a file path, a document's name, a URL query or the install ID.
- [ ] A test in the style of `use-native-menus.test.ts` shows `privacy.network-activity` is available with the desktop's actions.
- [ ] A browser check from the user's side: open the panel in each state (direct with updates on, direct with everything off, store build) and judge that the wording is true and readable. Record what was seen.

## Post-merge checks

- [ ] None.

## Tests

- Rust: the log records an update check and a report send with only the allowed fields; the store build compiles the log empty.
- TypeScript: the panel's states; `use-native-menus.test.ts` for availability.
- The web's network tests keep passing after any move.

## Docs to update

- `docs/plans/desktop-app.md`: tick the to-do under D9.
- The privacy page and `docs/decisions/0011-cloudflare-topology-and-privacy.md` if the panel adds a claim (the panel and the page must say the same).
- `docs/capabilities/README.md`: no row for the network panel; no change.
- `apps/desktop/CHANGELOG.md`.

## Risks and open questions

- **A claim the panel must keep true.** The panel shows only what the log records; any new outgoing request added later must go through the log, or the panel misleads. A test or lint that the updater and the reporter use one request helper keeps that so.
- **Store build review.** The listing says nothing about this panel, so it needs no new review answer; confirm with D8d's record before release.
- **Size.** If moving the web's monitor and panel is large, split the move first.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
