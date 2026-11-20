# Resolution: bookmark tests fail on Intel Macs

[Diagnosis](detail.md).

## The fix

- **Signed test binaries on Intel.** `laminapdf-desktop-os` and `laminapdf-desktop` have build scripts that share one rule (`adhoc_sign.rs`) and pass `-Wl,-adhoc_codesign` to the linker on `x86_64` macOS, as the Apple silicon linker does by default. Tauri's bundler signs a release build again, as it always did.
- **Errors, not `None`.** `file_bookmark` and `resolve_bookmark` return `io::Result`: `Unsupported` off macOS, `InvalidInput` for a path that isn't UTF-8, and otherwise the OS's error domain, code and description. The desktop app logs a warning when macOS refuses a bookmark and falls back to the path, as before.
- **Tests per kind.** The bookmark tests run with plain bookmarks everywhere; two more run with a security scope under `security-scoped`. Paths are canonicalised, since `/var` is `/private/var`.

## How it was checked

- `cargo nextest run -p laminapdf-desktop-os -p laminapdf-desktop`, with and without `security-scoped`, pass on Apple silicon.
- The cause is inferred, not reproduced; the nightly's `rust-macos-x64` job (run 44757463022, dispatched on the branch) will confirm or refute it on `macos-15-intel`.

## Left alone

Pull requests still skip the Intel Mac leg, so a break like this shows up in the nightly.
