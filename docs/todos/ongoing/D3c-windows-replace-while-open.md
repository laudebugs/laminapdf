# D3c — Let Windows apps replace a PDF that Lamina PDF has open

| Field | Value |
| --- | --- |
| ID | `D3c` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D3](../../plans/desktop-app.md#d3-files) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D3b](../done/D3b-recent-files-and-file-watching.md) |
| ADRs | Amends [ADR 0008 §2 and §11](../../decisions/0008-desktop-shell.md#11-recent-files-and-files-that-change-on-disk) |
| Branch | `fix/d3c-windows-replace-while-open` |
| PR title | `fix(desktop): let Windows apps replace a PDF that is open in Lamina PDF` |
| Size | M (300–700 lines) |
| PR | — |

## Goal

On Windows, an app that saves by renaming a new file over the old one with `MoveFileEx` or `ReplaceFile` fails while Lamina PDF has that PDF open. The main process's `FileSource` keeps a handle open for the document's whole life. It opens with `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`, which lets other apps write into the file, delete it, or replace it with a POSIX-semantics rename. The classic rename that `MoveFileEx` and `ReplaceFile` do still refuses a target that any process holds open. D3b's e2e scenario found it: Node's `renameSync`, which uses `MoveFileEx`, failed with `EPERM`. This todo makes that save work, so ADR 0008 §11's promise that Lamina PDF never blocks another app's save holds on Windows too.

## Context

- [ADR 0008 §11](../../decisions/0008-desktop-shell.md#11-recent-files-and-files-that-change-on-disk), "Open files stay readable", records the limitation; §2 describes `FileSource`.
- `apps/desktop/src-tauri/src/engine/bytesource.rs`: `FileSource`, positioned reads, the share flags. The engine reads the file through the main process in 256 KiB blocks (`RemoteSource`), so every read already goes through `FileSource`.
- `apps/desktop/src-tauri/src/watch/`: a replaced file already reloads as a new document under a new id. What has to change is only that the old document must not hold the old file open between reads.
- D3b's e2e scenario `should reload an atomic save and never say the file is missing` replaces the file through `e2e_replace` (Rust's POSIX-semantics rename) on Windows. Once this todo lands, it can use Node's rename on every OS.

## Scope

**In scope**

- On Windows, `FileSource` holds no handle between reads. The two options, to choose between by measurement:
  - open, read and close for every read;
  - close the handle after a short idle time (for example 2 s) and reopen it on the next read.
- A reopened file must be the same file. Compare its identity (`file-id`, as `FileStamp` does) with the one first opened. A different file fails the read as changed, never as the new file's bytes; the watcher's reload then shows the new document.
- The cost of opening, measured on a Windows runner with the 1 GB fixture: first page and a scroll through 1,000 pages, against the handle kept open today.
- The e2e atomic-save scenario through Node's rename on Windows too.
- ADR 0008 §2 and §11 amended.

**Out of scope**

- macOS and Linux, where a rename replaces an open file already.
- Network shares, which have their own rules (§11's last paragraph).

## Acceptance criteria

- [ ] On Windows, a `MoveFileEx(MOVEFILE_REPLACE_EXISTING)` replace of an open document succeeds, and the window reloads it (e2e, through Node's `renameSync`).
- [ ] A read after the file was replaced never returns the new file's bytes for the old document (unit test).
- [ ] First page and scroll times on Windows within 10 % of today's, or the difference recorded with the choice.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
