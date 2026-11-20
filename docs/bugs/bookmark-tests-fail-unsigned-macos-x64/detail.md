# Bookmark tests fail on Intel Macs

[Resolution](resolution.md).

## What we saw

The nightly's `Rust (macos-x64)` job (run 44729481527, runner `macos-15-intel`) failed three tests D8b (#761) added: `bookmark::tests::a_bookmark_should_find_its_file_after_a_rename` and `a_bookmark_of_a_deleted_file_should_find_nothing` in `laminapdf-desktop-os` (an `unwrap` on `None` while making a bookmark), and `recent::tests::a_moved_file_should_keep_its_entry` in `laminapdf-desktop` (the list held both `after.pdf` and `before.pdf`, because the entry had no bookmark to follow the move). They pass on Apple silicon, the only Mac a pull request tests.

## How to reproduce

Run `cargo nextest run -p laminapdf-desktop-os -p laminapdf-desktop --all-features` on an Intel Mac, or on `macos-15-intel`.

## Cause

`--all-features` turns on `security-scoped`, so the tests made security-scoped bookmarks. macOS refuses to make one for a process that isn't code-signed: the agent that issues it reads the caller's signature. On Apple silicon the linker signs every binary ad hoc, so the test binaries were signed. On Intel it doesn't, so the test binaries were not, and `bookmarkDataWithOptions` failed. `file_bookmark` turned that error into `None`, which hid the reason. I could not run an Intel binary here (no `x86_64-apple-darwin` target), so the signing is the diagnosis from the platform's behaviour, not a reproduction; the new code reports the OS's error, which will confirm it.

Every binary a user runs is signed, by the bundler or by the store, so no user was affected.
