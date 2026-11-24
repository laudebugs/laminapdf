# A recent file without a bookmark is unreachable in the sandboxed build

[Resolution](resolution.md). Issue #776, found in review of #774.

## What we saw

In the Mac App Store build (App Sandbox), a recent file that opened fine in one session could fail to open after a restart. The open failed with a message saying the file may have been moved or deleted, which was untrue.

## How to reproduce

Not reproduced on a sandboxed build, which this Mac can't run. The model reproduces in tests: with the sandbox on, an entry with no bookmark, or a bookmark that doesn't resolve, listed as a file with a handle.

## Cause

In the sandbox, a path reaches a file the user chose earlier only through its security-scoped bookmark. `scoped::bookmark` returns `None` when macOS refuses a bookmark (and logs a warning), and `recent.rs` then stored a path-only entry, as in the direct download, where a path works. `RecentFiles::list` reported every entry whose path was a file as having a handle and as not missing. `recent_open` then opened the path. In the sandbox that open failed, and the webview showed "couldn't be opened. It may have been moved or deleted".

Three cases lead there:

- macOS refuses to make the bookmark when the file is opened.
- A `recent.json` written before bookmarks existed holds path-only entries.
- A stored bookmark no longer resolves after a restart (stale, or its file changed volume).

The direct download is unaffected: a path works there.
