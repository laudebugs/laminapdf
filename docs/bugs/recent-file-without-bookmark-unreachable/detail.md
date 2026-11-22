# A recent file without a bookmark is unreachable in the sandboxed build

[Resolution](resolution.md). Issue #776, found in review of #774.

## What we saw

In the Mac App Store build (App Sandbox), a recent file that opened fine in one session could fail to open after a restart, or disappear from the list, with nothing said to the user.

## How to reproduce

Not reproduced on a sandboxed build: this Mac can't run one, and the Mac App Store build isn't signed yet. The model reproduces in tests: with the sandbox on, an entry with no bookmark, or a bookmark that doesn't resolve, listed as a file with a handle.

## Cause

In the sandbox, a path reaches a file the user chose earlier only through its security-scoped bookmark. `scoped::bookmark` returns `None` when macOS refuses a bookmark (and logs a warning), and `recent.rs` then stored a path-only entry, as in the direct download, where a path works. `RecentFiles::list` reported every entry whose path was a file as having a handle and as not missing, and `recent_open` opened the path. In the sandbox that open failed, and the webview showed "couldn't be opened. It may have been moved or deleted", which is untrue.

Three cases lead there:

- macOS refuses to make the bookmark when the file is opened.
- A `recent.json` written before bookmarks existed holds path-only entries.
- A stored bookmark no longer resolves after a restart (stale, or its file changed volume).

The direct download is unaffected: a path works there.
