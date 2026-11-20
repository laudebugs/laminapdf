# Resolution: the release engine links with fat LTO in one codegen unit

[Diagnosis](detail.md).

## The fix

`cargo xtask desktop-sidecar --release` (which every release build runs first through `desktop:build`) passes `--config profile.release.lto="fat"` and `--config profile.release.codegen-units=1` to the engine's `cargo build`. Only the engine takes them: the workspace's release profile stays thin, so no other release build gets slower, and Cargo keeps the two builds' artifacts apart.

The settings change how the code is optimised across crates, not what it does:

- **Crash reports.** `panic` is untouched, so the engine unwinds as before for the crash handler. The build action still sets `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` and, on Linux, `STRIP=none`, so `symbols.sh` still splits the debug information out for symbolication. `--config` does not touch either variable. Inlining across crates is wider, so a frame may be attributed to the caller it was inlined into.
- **The updater** is in the app shell, not the engine, and is built as before.

No budget change: the saving is about 695 KB gzipped against an overrun of 278 KB, and the plan's §4 row records the measurement.

Two other cuts were set aside. Raising the budget again only hides growth that a link setting can remove. Building the whole workspace with fat LTO would slow the CI lanes that build release code for no installer gain.

## How it was checked

The engine was built both ways and measured (table in the diagnosis). A unit test checks that `cargo_args` adds the settings for the shipped release engine and for no debug or end-to-end engine. The pull request's CI does not build the `.deb`; the Desktop release workflow does. Dispatch it with `gh workflow run "Desktop release" --ref fix/desktop-deb-size -f dry_run=true` (every target, nothing published) and read the Linux jobs' size table.

## Left for later

- The Linux and Windows sizes after the cut: only macOS Arm64 is measured, and fat LTO may save less on ELF. The dry-run Desktop release shows them.
- That `symbols.sh` still splits and links the debug information of the fat-LTO Linux build.
- The release engine's link time, which fat LTO in one codegen unit lengthens.

If the x86_64 `.deb` is still within 400 KB of the budget, the next step is a size look at the engine's largest crates (`cargo bloat`).
