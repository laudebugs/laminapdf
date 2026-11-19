# D8b — Mac App Store: ready the app for App Sandbox

| Field | Value |
| --- | --- |
| ID | `D8b` |
| Status | `ready` |
| Plan | [Desktop app §6 D8](../../plans/desktop-app.md#d8-mac-app-store) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D8a](../done/D8a-mac-app-store-plan.md) |
| ADRs | Implements [0014 §11](../../decisions/0014-desktop-distribution.md#11-the-mac-app-store-channel); follows [0008](../../decisions/0008-desktop-shell.md) §2 and §5 |
| Branch | `feat/d8b-app-sandbox-readiness` |
| PR title | `feat(desktop): run in App Sandbox for the Mac App Store build` |
| Size | M (200–400 lines of code; tests beyond) |
| PR | — |

## Goal

The desktop reader works whole in App Sandbox, as the Mac App Store requires, behind a build feature the store build turns on. Every file reaches it through the open and save panels, Finder, drag and drop, or a security-scoped bookmark, and the direct download behaves exactly as before.

## Context

- ADR 0014 §11 (the entitlements), ADR 0008 §2 and §5 (the engine already runs sandboxed on macOS)
- [Apple: App Sandbox](https://developer.apple.com/documentation/security/app-sandbox), [security-scoped bookmarks](https://developer.apple.com/documentation/foundation/nsurl/bookmarkdata(options:includingresourcevaluesforkeys:relativeto:)), [Tauri: App Store](https://v2.tauri.app/distribute/app-store/)
- The desktop shell research, [Q4](../../research/architecture/notes/desktop-shell.md#q4-file-system-file-associations-drag--drop-printing-sandboxing-multi-window)

## Scope

- In: an `app-store` Cargo feature in the main process; `Entitlements.appstore.plist` (app sandbox, user-selected read-write, print, network client) and the engine's (sandbox and `inherit`); security-scoped bookmarks for recent files and session restore, behind the feature, with the paths kept for the direct download; starting and stopping access around every read and save; the file watcher, folder writes (E2u) and the crash monitor checked in the sandbox; a sandboxed dev build to try it.
- Out: the store build's configuration, signing and upload ([D8c](D8c-app-store-build-and-upload.md)); the listing and review ([D8d](D8d-app-store-listing-and-first-review.md)).

## Tasks

- [ ] Fresh worktree; claim in the work PR
- [ ] The `app-store` feature and the two entitlements files
- [ ] Bookmarks: recent files and the session store keep them; reopening resolves them, refreshes stale ones and drops those the user can't reach any more
- [ ] Access scopes around reads, saves, the watcher and folder writes
- [ ] The crash monitor and the engine in the sandbox (the monitor's socket, minidump folder)
- [ ] A sandboxed local build (ad-hoc signed with the store entitlements) and the reader tried in it, by hand: open from the panel and from Finder, drag and drop, recent files after a relaunch, session restore, print, save a copy, a folder of parts

## Acceptance criteria

- [ ] In the sandboxed build, every task above works as in the direct download, and `codesign -d --entitlements` lists exactly ADR 0014 §11's entitlements
- [ ] Recent files and session restore reopen files after a relaunch; a moved or deleted file is dropped with the same message as today
- [ ] The direct download's behaviour and tests are unchanged

## Docs to update

- ADR 0008 §5: the sandbox the store build runs in
- `docs/agents/layout.md` and `docs/agents/commands.md`: the feature and the sandboxed local build

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
