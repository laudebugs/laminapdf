# The nightly desktop release failed: the Linux x86_64 .deb was over 16 MB

## What we saw

The scheduled Desktop release (run 44741935127, job "Nightly (linux-x86_64)") failed at `cargo xtask desktop-release sizes`:

```
Lamina PDF_0.1.0-nightly.20261120.92_amd64.deb is 16278410 bytes, over the 16 MB budget
```

The budget is `BUDGET` in `tools/xtask/src/desktop_release.rs`, 16,000,000 bytes, the desktop plan's installer size ([§4](../../plans/desktop-app.md#4-requirements)). The owner raised it from 15 MB on 2026-11-15 for the crash reader. The nightly before this one (run 44533787062, 2026-11-19) failed the same way at 16,218,380 bytes.

## What grew

The code of the desktop engine, the sidecar the app bundles. The Linux `.deb`s of the scheduled runs:

| Nightly | x86_64 | Arm64 | Merged since the one before |
| --- | ---: | ---: | --- |
| 2026-11-18 (success) | 15,872,046 | 15,325,000 | |
| 2026-11-19 (failed) | 16,218,380 | 15,642,010 | the save planner's checks of authoring ops against field locks and usage rights (E9k, #754), D8b's App Sandbox build (#761), the Finder cold-open fix (#763) |
| 2026-11-20 (failed) | 16,278,410 | 15,690,364 | form flattening (E8p, #756) and Fill & Sign (E8e, #770) |

The growth is in the engine: PDFium and the app shell did not change, and E9k, the largest of these merges, adds code the engine links. Both steps were small ones, so what failed was the missing headroom: the x86_64 `.deb` was 128 KB under the budget on 2026-11-18, and every merge that adds engine code spends it.

## The cause

The workspace's release profile links with thin LTO and 16 codegen units, for build times. Each of the 16 units is optimised on its own, so code that two crates share is inlined and duplicated across units. A change that adds engine code adds it with that overhead, and the installer had 128 KB to spare on x86_64.

## Measurements

The engine on macOS Arm64, stripped, built from this commit with `cargo build --release -p laminapdf-desktop-engine`:

| Link settings | Bytes | gzip -9 |
| --- | ---: | ---: |
| thin LTO, 16 codegen units (the workspace's release profile) | 9,173,472 | 3,609,127 |
| fat LTO, one codegen unit | 6,195,968 | 2,914,255 |

Fat LTO saves 2,977,504 bytes raw and 694,872 gzipped. The `.deb` compresses its contents, so the gzipped figure is the better guide: about 695 KB against an overrun of 278,410 bytes on x86_64. The Linux engine differs from the macOS one, so the exact Linux saving is for the next Desktop release run to show.
