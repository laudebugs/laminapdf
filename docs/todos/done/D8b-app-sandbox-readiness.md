# D8b — Mac App Store: ready the app for App Sandbox

| Field | Value |
| --- | --- |
| ID | `D8b` |
| Status | `done` |
| Plan | [Desktop app §6 D8](../../plans/desktop-app.md#d8-mac-app-store) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D8a](./D8a-mac-app-store-plan.md) |
| ADRs | Implements [0014 §11](../../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build); follows [0008](../../decisions/0008-desktop-shell.md) §2 and §5 |
| Branch | `feat/d8b-app-sandbox-readiness` |
| PR title | `feat(desktop): run in App Sandbox for the Mac App Store build` |
| Size | M (200–400 lines of code; tests beyond) |
| PR | [#761](https://github.com/laudebugs/laminapdf/pull/761) |

## Goal

The desktop reader works whole in App Sandbox, as the Mac App Store requires, behind a build feature the store build turns on. Every file reaches it through the open and save panels, Finder, drag and drop, or a security-scoped bookmark, and the direct download behaves exactly as before.

## Context

- ADR 0014 §11 (the entitlements), ADR 0008 §2 and §5 (the engine already runs sandboxed on macOS)
- [Apple: App Sandbox](https://developer.apple.com/documentation/security/app-sandbox), [security-scoped bookmarks](https://developer.apple.com/documentation/foundation/nsurl/bookmarkdata(options:includingresourcevaluesforkeys:relativeto:)), [Tauri: App Store](https://v2.tauri.app/distribute/app-store/)
- The desktop shell research, [Q4](../../research/architecture/notes/desktop-shell.md#q4-file-system-file-associations-drag--drop-printing-sandboxing-multi-window)

## Scope

- In:
  - an `app-store` Cargo feature in the main process;
  - the app's entitlements (app sandbox, user-selected read-write, app-scoped bookmarks, print, and outgoing connections, which WKWebView needs; the app's own code makes no request, owner 2026-11-19, #764) and the engine's (`app-sandbox` and `inherit` only), in `apps/desktop/src-tauri/app-store/`;
  - the engine in App Sandbox: a descriptor cap in place of the Seatbelt profile's network rule, and its own memory watch;
  - Settings in the store build: updates come from the App Store, and crash reports go through macOS;
  - security-scoped bookmarks for recent files and session restore, behind the feature, with the paths kept for the direct download, and whether they need `com.apple.security.files.bookmarks.app-scope`;
  - starting and stopping access around every read and save;
  - the file watcher, folder writes (E2u: a folder the user picks) and the crash monitor, checked in the sandbox;
  - whether the engine can still narrow its sandbox with its own profile (ADR 0008 §5) once it inherits the app's;
  - a sandboxed local build to try it.
- Out: the store build's configuration, signing and upload ([D8c](../ongoing/D8c-app-store-build-and-upload.md)); the listing and review ([D8d](../ready/D8d-app-store-listing-and-first-review.md)).

## Tasks

- [x] Fresh worktree; claim in the work PR
- [x] The `app-store` feature and the two entitlements files (the app keeps `network.client`, which WKWebView needs: owner, #764)
- [x] Bookmarks: recent files and the session store keep them; reopening resolves them, refreshes stale ones (a fresh bookmark each time a file opens) and drops those the user can't reach any more
- [x] Access scopes around reads, saves, the watcher and folder writes: access is held from resolving until quit; whole-file saves write in macOS's replacement folder. Folder writes wait for E2u, which isn't in the desktop yet
- [x] The crash monitor and the engine in the sandbox: the store build has no crash monitor or reports of its own; the engine inherits App Sandbox, caps its descriptors and watches its own memory
- [x] A sandboxed local build (ad-hoc signed with the store entitlements) and the reader tried in it, by hand on macOS 27 (2026-11-19): open from the panel and from Finder (cold and warm), drag and drop, recent files after a relaunch, session restore, print to PDF, save a copy, save as (new file and over one), save in place, and a file changed on disk reloading. A folder of parts waits for E2u

## Acceptance criteria

- [x] In the sandboxed build, every task above works as in the direct download, and `codesign -d --entitlements` lists ADR 0014 §11's entitlements (the provisioning profile's identifiers come with D8c's store signing)
- [x] Recent files and session restore reopen files after a relaunch; a moved or deleted file is dropped with the same message as today
- [x] The direct download's behaviour and tests are unchanged

## Docs to update

- ADR 0008 §5: the sandbox the store build runs in
- `docs/agents/layout.md` and `docs/agents/commands.md`: the feature and the sandboxed local build

## Definition of done

- [x] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
