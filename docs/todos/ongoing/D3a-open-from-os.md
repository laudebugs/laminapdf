# D3a — Open from the OS: file associations, argv, `RunEvent::Opened`, single instance, drag-drop

| Field | Value |
| --- | --- |
| ID | `D3a` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D3](../../plans/desktop-app.md#d3-files) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D2e](../done/D2e-desktop-e2e-webdriverio.md) |
| ADRs | Follows ADR 0008 (paths only enter through trusted sources), ADR 0010 (Platform adapter) |
| Branch | `feat/d3a-open-from-os` |
| PR title | `feat(desktop): open PDFs from file associations, argv, the dock and drag-drop` |
| Size | M–L (500–900 lines) |
| PR | — |

## Goal

Make lamina behave like a native PDF app: it registers for `.pdf` in every installer, double-clicking or "Open with" opens the file in the **running** instance (a new instance forwards its paths and exits), files dropped on the window or on the macOS dock icon open, and a cold start with a file argument opens it straight away. This is functional requirement 1 of the [desktop plan](../../plans/desktop-app.md#functional) and the main reason to have a desktop reader at all ([§3 users](../../plans/desktop-app.md#3-users-and-use-cases): "Default PDF app that opens instantly from Finder/Explorer/Files").

## Context

- **Design (Paper):** [S01 Start window](../../design/paper/desktop-shell.md#s01-start-window). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [Desktop notes Q4](../../research/architecture/notes/desktop-shell.md#q4-file-system-file-associations-drag--drop-printing-sandboxing-multi-window): `RunEvent::Opened` on macOS (cold start: store URLs in state; running: emit); on Windows/Linux paths arrive as argv to a new process, so `tauri-plugin-single-instance` forwards them; single-instance fixed the macOS cwd of the second instance (v2.2.4).
- Tauri: [`bundle.fileAssociations`](https://v2.tauri.app/reference/config/#fileassociation) (`ext`, `mimeType`, `name`, `description`, `role`, `rank` on macOS; Windows registry via NSIS/WiX; Linux `.desktop` `MimeType=`), [single-instance plugin](https://v2.tauri.app/plugin/single-instance/) (must be the **first** plugin registered), [file associations example](https://v2.tauri.app/learn/mobile-file-associations/), `WindowEvent::DragDrop(DragDropEvent::Drop { paths, position })`.
- D2b's `DocumentRegistry`: the only way paths become documents; D2d's open flow.
- [Plan open question 5](../../plans/desktop-app.md#8-open-questions): register as default PDF handler on install (opt-in prompt) or leave to the user — this PR implements "register as a handler, never steal the default" and records the decision.

## Scope

**In scope**

- `tauri.conf.json` `bundle.fileAssociations`: `[{ "ext": ["pdf"], "mimeType": "application/pdf", "name": "PDF Document", "description": "Portable Document Format", "role": "Viewer", "rank": "Alternate" }]` (macOS `rank: Alternate` so we don't claim the default; Windows/Linux equivalents are "Open with" registrations).
- `tauri-plugin-single-instance` registered first: callback receives `(app, argv, cwd)`, resolves relative paths against `cwd`, filters to existing `.pdf` files (case-insensitive) and `file://` URLs, focuses/unminimises the target window and passes the paths to `route_open` (a second launch can arrive while the first instance is still starting, so it must use the queue too).
- **One entry point for every OS open request**: `route_open(app, paths)`. Until the frontend reports ready, it appends to a `PendingOpens` state (a `Mutex<PendingOpens { ready: bool, paths: Vec<PathBuf> }>` so the check and the push are atomic); the frontend calls `take_pending_opens` once after mount, which drains the queue and sets `ready`; afterwards `route_open` opens directly. All three sources below go through it — none opens a document itself:
  - Cold-start argv (Windows/Linux, and macOS when launched from a terminal): parsed in `setup` (skip `argv[0]`, ignore flags such as `-psn_*` on old macOS and Tauri/WebDriver flags) and passed to `route_open`, which queues them because the webview hasn't mounted yet.
  - The single-instance callback (a second launch while running).
  - macOS `RunEvent::Opened { urls }` handled in `App::run(|app, event| …)` with `#[cfg(target_os = "macos")]` (cold start before or after `setup`, warm opens, Dock-icon drops).
- Window drag-drop: handle `WindowEvent::DragDrop(Drop { paths, .. })` in Rust (not in JS), open PDFs, ignore others with a toast; show a drop overlay via `DragDropEvent::Enter/Leave` forwarded as events. `dragDropEnabled` stays `true` on document windows.
- Open request routing: until tabs exist (D4a), opening while a document is open replaces it in the focused window after confirming nothing is lost (reader: no prompt needed) — or opens a new window if `multiWindow` already supported; record the interim behaviour.
- Error handling: unreadable/missing path → toast with the file name only (no full path in logs at `info`).
- Decision on open question 5 recorded in ADR 0008 (amendment) or the plan.
- e2e scenarios (WebdriverIO, D2e harness): cold start with a path argument opens it; a second launch with a path forwards to the first instance (Windows/Linux: launch the binary again with the path; macOS: `open -a <app> file.pdf` in the manual checklist).

**Out of scope** (and which todo covers it)

- Recent files, OS recent documents, file watching: [D3b](../ready/D3b-recent-files-and-file-watching.md).
- Tabs and multi-window routing: [D4a](../ready/D4a-windows-tabs-and-session-restore.md).
- Installer-level verification of associations (MSI/NSIS/deb registration on clean VMs): [D6b](../ready/D6b-release-workflow-and-linux-packages.md)–[D6d](../ready/D6d-windows-signing.md) (this PR verifies with dev bundles).
- Custom URL scheme deep links (`lamina://` for the web ↔ desktop handoff): not planned for D-rounds.

## Tasks

- [ ] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2` and `rust-best-practices`.
- [ ] `cargo tauri add single-instance` equivalent: add `tauri-plugin-single-instance` to workspace deps (licence check); register first in the builder. No webview permission needed (Rust-only).
- [ ] `src-tauri/src/open.rs`: `fn paths_from_args(args: &[String], cwd: &Path) -> Vec<PathBuf>` (pure, unit-tested), `fn route_open(app: &AppHandle, paths: Vec<PathBuf>)`, `PendingOpens` state, `take_pending_opens` command (manifest + capability).
- [ ] macOS `RunEvent::Opened` handler converting `Url` → `PathBuf` (`to_file_path()`), skipping non-file URLs.
- [ ] Drag-drop handling in `on_window_event`; events `drop://enter|leave` to the UI; overlay component in `@laminapdf/ui` (shared, behind `Platform` capability `acceptsFileDrop` if needed — web already supports drop via DOM, so reuse the same overlay).
- [ ] `bundle.fileAssociations` config; macOS `Info.plist` check (`CFBundleDocumentTypes` emitted by Tauri — verify `LSHandlerRank`); Linux `.desktop` file `MimeType=application/pdf;` and `Exec=lamina %F` (verify Tauri's generated `Exec` passes files; override with a custom desktop template via `bundle.linux.deb.desktopTemplate` if not).
- [ ] Windows: verify NSIS and WiX register `OpenWithProgids`/`Applications\lamina.exe\SupportedTypes` and the `%1` argument quoting for paths with spaces and Unicode.
- [ ] e2e scenarios + manual checklist (`apps/desktop/e2e/MANUAL-OPEN.md`) covering: double-click in Finder/Explorer/Files, "Open with", dock drop, taskbar-pinned app with "Open with", paths with spaces/Unicode/very long paths (Windows `\\?\`), network share.
- [ ] Record the default-handler decision.

## Acceptance criteria

- [ ] On each OS, with a dev bundle installed (dmg/NSIS/deb), double-clicking a PDF when lamina is **running** opens it in the running instance (no second window process remains — checked with the process list) — manual check recorded in the PR per OS.
- [ ] Cold start via double-click or `laminapdf /path/to/file.pdf` opens the file on every OS through `PendingOpens` (no code path opens a document before the frontend has called `take_pending_opens`); first page within the cold-start + first-page budgets.
- [ ] Dropping a PDF onto the window opens it; dropping onto the macOS dock icon opens it; dropping a non-PDF shows a toast and opens nothing.
- [ ] Paths with spaces, non-ASCII characters and (Windows) > 260 characters open correctly (unit tests for parsing + manual check).
- [ ] lamina appears in "Open with" on all three OSes and does **not** become the default handler unless the user chooses it.
- [ ] New e2e scenarios pass in `desktop-e2e`.

## Post-merge checks

None.

## Tests

- Unit: `paths_from_args` (relative paths, `file://` URLs, flags, non-PDF filtering, duplicates, Windows drive-relative paths).
- Unit: `PendingOpens` queue drained once; opens after ready bypass the queue; cold-start argv paths and single-instance paths received before `take_pending_opens` are queued (not opened, not dropped) and come out in arrival order.
- e2e (WebdriverIO): cold start with path arg; second-instance forwarding (Windows/Linux) using the e2e harness to launch the binary again.
- Manual checklist per OS stored in the repo and linked from the PR.

## Docs to update

- `docs/plans/desktop-app.md` §9 D3: tick "Register `.pdf` file associations…", "Handle argv, `RunEvent::Opened` and single-instance forwarding", and the open-from-OS half of "Integration tests for open-from-OS on each platform"; answer §8 Q5 (link).
- ADR 0008 amendment (default-handler policy) if recorded there.
- `apps/desktop/README.md`: testing associations locally.

## Risks and open questions

- **Tauri's generated Linux `.desktop` `Exec` line** may not include `%F`/`%U`; verify and template if needed.
- **macOS `RunEvent::Opened` before `setup` finishes** — the queue handles ordering; test with a cold double-click.
- **Windows "Open with" for MSI vs NSIS** may register differently (per-machine vs per-user); align with D6d's install scope choice.
- **Single-instance on Linux** uses D-Bus; snaps/flatpaks out of scope; sandboxed environments without a session bus fall back to multiple instances — acceptable, log it.
- **Default-handler prompt** (open question 5): the owner may prefer an opt-in prompt on first run; if so, add it in D7b's settings work.

## Notes

What the PR changed from the scope above, and why:

- **Missing files are reported, not filtered out.** `paths_from_args` doesn't check that files exist, so a missing one reaches the window as a toast with its name (the Error handling item), not silently dropped. The one exception: a flag-like argument is kept only when it is an existing file (`-scan.pdf`).
- **Event names:** `open://drag-enter` and `open://drag-leave` in place of `drop://enter|leave`, beside `open://document` and `open://refused` (dropped files that aren't PDFs), so every open event shares one prefix.
- **Only a missing or unreadable file is a toast.** A file the engine refuses (damaged, not a PDF inside) goes to the failure page, as from the open dialog, which says why.
- **Several files at once open the last**, since a window shows one document until tabs (D4a). The OS's requests open one at a time, in the order they came, on one worker thread.
- **Windows' default handler moved to D6d.** Tauri's NSIS and WiX templates register an open command with `"%1"` quoted (read in the templates at the CLI's version, 2.12.4) and also write their class as `.pdf`'s default value. Windows 10 and 11 ignore that value when the user has chosen a default, so it decides only on a machine where nobody chose one. Changing the templates to register through `OpenWithProgids` alone is installer work, so [D6d](../ready/D6d-windows-signing.md) has it, with this todo's check that both installers register "Open with".
- **Manual per-OS checks with installed bundles** are listed in `apps/desktop/e2e/MANUAL-OPEN.md`. The macOS bundle's `Info.plist` was read (`com.adobe.pdf`, Viewer, `LSHandlerRank` `Alternate`). The installed-bundle runs on Windows and Linux were not done: they need those machines, and D6b–D6d's clean-VM checks already cover the installers' registration.
- **The cold start with a file reports, never asserts, its time** to the first sharp page (ADR 0018 keeps wall-clock checks out of the PR lanes).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
