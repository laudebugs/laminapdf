# Bookmark tests fail on Intel Macs

[Resolution](resolution.md).

## What we saw

The nightly's Intel Mac job (`Rust (macos-x64)`, run 44729481527, runner `macos-15-intel`) failed three bookmark tests that pass on Apple silicon, the only Mac a pull request tests. D8b (#761) added them:

- `laminapdf-desktop-os` `bookmark::tests::a_bookmark_should_find_its_file_after_a_rename` and `a_bookmark_of_a_deleted_file_should_find_nothing`: an `unwrap` on `None` while making a bookmark.
- `laminapdf-desktop` `recent::tests::a_moved_file_should_keep_its_entry`: the list held both `after.pdf` and `before.pdf`, because the entry had no bookmark to follow the move.

## How to reproduce

Run `cargo nextest run -p laminapdf-desktop-os -p laminapdf-desktop --all-features` on an Intel Mac, or on `macos-15-intel`.

## Cause

Inferred, not reproduced: an unsigned process can't make a security-scoped bookmark. The agent that issues one reads the caller's code signature and refuses an unsigned caller.

`--all-features` turns on `security-scoped`, so the tests made security-scoped bookmarks. On Apple silicon the linker signs every binary ad hoc, so the test binaries were signed. On Intel it doesn't, so they were not, and `bookmarkDataWithOptions` failed. `file_bookmark` turned that error into `None`, which hid the reason.

No `x86_64-apple-darwin` target was available to run an Intel binary. Run 44757463022's macOS x64 job will test the fix, and the OS's error will confirm or refute this.

Every binary a user runs is signed, by Tauri's bundler or by the store, so no user was affected by the signing itself.
