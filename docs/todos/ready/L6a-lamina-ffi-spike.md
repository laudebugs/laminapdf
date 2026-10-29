# L6a — `lamina-ffi` spike (start right after B5c): UniFFI over the façade, XCFramework and AARs, size report, ADR 0029

| Field | Value |
| --- | --- |
| ID | `L6a` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L6](../../plans/shareable-libraries.md#l6-mobile), principles [§4](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031) (1, 4), open questions [§11](../../plans/shareable-libraries.md#11-open-questions) (1, 2) |
| Round / milestone | Round L6 (shareable libraries) — Mobile |
| Depends on | [B5c](./B5c-lamina-facade-and-examples.md), [L1a](../ongoing/L1a-contract-adrs.md) |
| Blocks | [L6b](./L6b-mobile-sdk-plan.md) |
| ADRs | Writes **0029** (mobile bindings and packaging); fills in [0002 §5](../../decisions/0002-binding-strategy.md#decision) without changing it; follows 0028 (façade rules, jobs, cancellation) and 0026 (errors) from [L1a](../ongoing/L1a-contract-adrs.md), [0001](../../decisions/0001-core-language-and-licence.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `spike/l6a-lamina-ffi` |
| PR title | `spike(ffi): bind the lamina façade with UniFFI for Swift and Kotlin; ADR 0029` |
| Size | L (spike code ≈ 600–1,000 lines incl. Swift/Kotlin tests and scripts; ADR and README extra) |
| PR | — |

## Goal

**Timing (decided (project owner, 2026-10-29)):** start as soon as [B5c](./B5c-lamina-facade-and-examples.md) merges, in parallel with B6, so façade problems surface before `@lamina/pdf` fixes the TypeScript API.

Answer one question with measurements before any mobile SDK work: **does the B5c façade bind to Swift and Kotlin through UniFFI without changes?** Bind open, page count and sizes, extraction and save; build an XCFramework (via cargo-swift) and AARs (via cargo-ndk); run the same scenario from Swift and Kotlin tests; prototype cancellation on top of ADR 0028's `CancelToken`; report sizes per ABI. Record the binding and packaging decisions in ADR 0029 (UniFFI pin, idiomatic Swift/Kotlin layer, SPM only, Maven Central AARs, 16 KB page alignment, panic profile).

**Recommendation: spike first.** Put the code in `spikes/lamina-ffi/` (outside both workspaces, like `spikes/krilla-fit/`, depending on `crates/lamina` by path). A product `crates/lamina-ffi` edge crate comes from the mobile SDK plan ([L6b](./L6b-mobile-sdk-plan.md)) once ADR 0029 is accepted. Reasons: the UniFFI pin is still open, the toolchain targets and CI runners (macOS, Android NDK) are not in the workspace yet, and a spike can be thrown away if the façade needs reshaping. Decide in the PR; if the façade binds cleanly and the CI cost is small, promoting straight to `crates/lamina-ffi` is acceptable.

## Context

- [U3b](./U3b-mobile-shell-spike.md) (`mobile-shell` spike, ADR 0025) decides how lamina's **own** mobile apps are built (Tauri 2 mobile or UniFFI with native UI). This spike is about the **SDK for other developers**, which needs UniFFI whatever U3b decides; its size and binding measurements are an input to U3b, and the two spikes should share the test device matrix.
- [Report: design the façade to the stricter binding now](../../research/libraries/shareable-libraries.md#design-the-façade-to-the-stricter-binding-now): UniFFI's `Arc`/`&self`/`Send + Sync` rules, Bitwarden's and Matrix's thin leaf crates per binding, no generics; the 0.31 vs 0.32 trade-off.
- [Report: cancellation and progress](../../research/libraries/shareable-libraries.md#cancellation-and-progress-belong-in-laminas-api-not-the-bindings) and [concurrency: generated bindings need a hand-written idiomatic layer](../../research/libraries/shareable-libraries.md#concurrency-generated-bindings-need-a-hand-written-idiomatic-layer): no built-in cancellation; Swift `withTaskCancellationHandler`; Kotlin coroutine cancellation plus `Drop`; `@unchecked Sendable`.
- [Report: packaging: SPM only, AARs on Maven Central](../../research/libraries/shareable-libraries.md#packaging-spm-only-aars-on-maven-central): CocoaPods trunk read-only on 2 December 2026; cargo-swift 0.11.1; `lamina-core` and `lamina-compose` AARs; 16 KB alignment.
- [Report: panic safety](../../research/libraries/shareable-libraries.md#panic-safety-no-panics-in-core-a-disposable-instance-on-the-web-unwinding-at-native-edges): UniFFI catches panics only under `panic=unwind` (unverified), so mobile profiles must not inherit `wasm-release`'s `panic=abort`.
- [Rust core bridge note §1](../../research/libraries/notes/rust-core-bridge.md#1-uniffi-in-2026-version-features-packaging-real-world-users) (UniFFI 0.32.2 features, ubrn pinned to `=0.31`, XCFramework and cargo-ndk packaging, JNA overhead), [§2](../../research/libraries/notes/rust-core-bridge.md#2-one-façade-for-wasm-bindgen-and-uniffi-patterns-and-pitfalls) (façade sketch and pitfalls), [§5](../../research/libraries/notes/rust-core-bridge.md#5-concurrency-cancellation-progress), [§8](../../research/libraries/notes/rust-core-bridge.md#8-panic-safety-across-ffi).
- [Mobile SDK note §5](../../research/libraries/notes/mobile-sdk.md#5-native-concurrency-and-uniffi-async) and [§6](../../research/libraries/notes/mobile-sdk.md#6-packaging-and-distribution).
- [ADR 0002](../../decisions/0002-binding-strategy.md) (§3 handle-based façade, §5 UniFFI later, pinned; 0.32 changed Kotlin `ForeignBytes`), [code organization §12 spikes vs product code](../../guidelines/code-organization.md#12-spikes-vs-product-code).

## Scope

**In scope**

- `spikes/lamina-ffi/` (standalone Cargo project with its own `[workspace]`, `publish = false`, licence `MIT OR Apache-2.0`): a `cdylib` + `staticlib` with UniFFI proc-macro exports wrapping `lamina`:
  - `FfiDocument` (`#[derive(uniffi::Object)]` around `lamina::Document`): `open(bytes, options) -> Arc<FfiDocument>`, `page_count()`, `page_sizes() -> Vec<PageSize>` (top-left points per ADR 0027), `info()`, `text(page)` when B9b is merged (else metadata only, noted in the README), `set_info(key, value)` through the op log, `save(options) -> SaveResult` (full bytes, and the appended section only for incremental saves).
  - Records and enums for options, sizes, warnings; one `FfiError` (`#[derive(uniffi::Error)]`) mapping `lamina::Error` with ADR 0026's `code`, `kind`, `message`, `hint`.
  - A job prototype: `FfiJob` with `step(budget_ms) -> FfiStep` and `cancel()` over ADR 0028's job if B5c has one, else a toy job over all pages.
- **UniFFI pin: 0.32** (decided (project owner, 2026-10-29); [roadmap: decisions made](../../plans/roadmap.md#decisions-made)). Pin an exact `=0.32.x`. Exercise what 0.32 adds: zero-copy `&[u8]` input, foreign traits (`#[uniffi::export(foreign)]`, needed if Swift/Kotlin implement [L3c](./L3c-model-plugins.md)'s `OcrEngine` or a platform signer) and async constructors. 0.31 is not built; React Native (ubrn, which supports only `=0.31` today) is revisited when it is scheduled.
- **iOS:** `cargo swift package` (cargo-swift, pinned) producing `Lamina.xcframework` (ios-arm64, ios-arm64-simulator; macOS slice optional for host tests) inside a Swift package with a binary target plus a small hand-written source target (`LaminaDocument` with `async` methods; `withTaskCancellationHandler` calling `job.cancel()`); XCTest tests for open → page count → text → `set_info` → save → reopen.
- **Android:** `cargo ndk` (pinned) for arm64-v8a and x86_64 (armeabi-v7a measured, recommendation to drop), linked with `-Wl,-z,max-page-size=16384`; Kotlin bindings plus a small hand-written layer (`suspend` functions, `suspendCancellableCoroutine` with `invokeOnCancellation { job.cancel() }`); JVM host tests with the host `.so` (UniFFI Kotlin over JNA runs on desktop JVM), and one instrumented emulator test if practical; an AAR assembled by a minimal Gradle project.
- **Panic profile:** `[profile.mobile-release]` (inherits `release`, `panic = "unwind"`, `lto`, `codegen-units = 1`, `opt-level` measured `s` vs `z`, `strip`); a test-only export that panics proves the panic surfaces as a Swift error and a Kotlin exception instead of aborting.
- **Size report** per ABI and slice: stripped `.so` and static-lib contribution, XCFramework zip, AAR, and estimated app-size delta (App Store thinning, Play ABI splits) for `lamina` with and without the `text` feature; JNA's extra size noted.
- **Checks:** `llvm-readelf -l` shows `LOAD` alignment `0x4000` for every `.so` (or `zipalign -c -P 16` on the AAR).
- **ADR 0029** `docs/decisions/0029-mobile-bindings-and-packaging.md`: UniFFI pin and upgrade policy; leaf-crate layout (`crates/lamina-ffi` edge crate, binding code only); where UniFFI derives live (wrapper types in `lamina-ffi` vs `cfg_attr` derives in core data types behind a `uniffi` feature, which the portability rules must allow or reject); idiomatic Swift/Kotlin layer rules (Sendable handles, `async`/`suspend`, `AsyncStream`/`Flow`, cancellation through `CancelToken`); packaging (SPM binary target + source target only, no CocoaPods; Maven Central `lamina-core` AAR, later `lamina-compose`; ABIs; minimum iOS and Android versions; 16 KB alignment); panic profile; per-ABI size budgets set from the measurements.

**Out of scope** (and which todo covers it)

- The product `crates/lamina-ffi`, release pipelines, Maven Central and SPM publishing: rounds from [L6b](./L6b-mobile-sdk-plan.md) (and [P1](./P1-reserve-names-and-trusted-publishing.md) for name reservation).
- Rendering and tiles (`render_tile`, `trim`): need [C2b](./C2b-lamina-render-crate.md); planned in [L6b](./L6b-mobile-sdk-plan.md).
- SwiftUI and Compose views: [L6b](./L6b-mobile-sdk-plan.md).
- React Native (ubrn) and Kotlin Multiplatform (Gobley): measured only as "does the pin allow it"; no bindings built.
- Adding mobile jobs to the PR lane: the spike is not in CI (like `spikes/krilla-fit/`); ADR 0029 states the CI plan for the product crate.

## Tasks

- [ ] Load `/rust-best-practices` and follow [docs/guidelines/rust.md](../../guidelines/rust.md) (the spike is throwaway-quality but keeps no `unwrap` in exported paths; every `unsafe`, if any, has `// SAFETY:`); read the Context links and ADR 0002, 0028
- [ ] Decide spike vs edge crate (recommendation above) and say so in the PR description
- [ ] Create `spikes/lamina-ffi/{Cargo.toml,src/lib.rs,uniffi.toml,README.md,scripts/}`; pin `uniffi`, cargo-swift and cargo-ndk versions; licence-check new dependencies (`cargo deny` in the spike)
- [ ] `scripts/build-ios.sh`, `scripts/build-android.sh`, `scripts/size-report.sh` (writes a Markdown table into the README), `scripts/check-alignment.sh`
- [ ] Swift package `spikes/lamina-ffi/swift/` with XCTest; Kotlin/Gradle project `spikes/lamina-ffi/kotlin/` with JVM tests
- [ ] Build and test with UniFFI `=0.32.x`; confirm zero-copy `&[u8]` input reaches Rust without a copy in the generated Swift and Kotlin code
- [ ] Cancellation prototype on both platforms; **verify Kotlin cancellation semantics in UniFFI 0.32** (does cancelling the coroutine drop the Rust future, and does our `Drop` set the flag?) with a test that observes the flag
- [ ] Panic test on both platforms under `mobile-release`
- [ ] List every façade change UniFFI forced (expected: none if ADR 0028 holds); file each as a follow-up against B5c if any
- [ ] Write ADR 0029; add it to `docs/decisions/README.md` and the [architecture §3 table](../../plans/architecture.md#3-decisions-to-record-adrs)
- [ ] Spike README: question, answer, measurements, how to reproduce (rustup targets `aarch64-apple-ios-sim`, `x86_64-linux-android`, etc. installed by the scripts, not added to the root `rust-toolchain.toml` unless ADR 0029 decides so); add the spike to `CLAUDE.md` and the spikes list

## Acceptance criteria

- [ ] The spike README answers "does the façade bind without changes?" with a yes or a list of the changes needed, each with the UniFFI rule that forced it
- [ ] Swift XCTest and Kotlin JVM tests pass for open → page count → page sizes → text (or info) → `set_info` → save → reopen, and the saved bytes are identical between Swift, Kotlin and Rust for the same input
- [ ] Cancelling a running job from Swift (`Task.cancel()`) and Kotlin (`job.cancel()`) stops it within one step and surfaces `Cancelled`; the Kotlin drop behaviour under the chosen pin is recorded
- [ ] A deliberate panic in a test export becomes a Swift error and a Kotlin exception under `mobile-release` (not a process abort)
- [ ] Size table per ABI and slice, with and without `text`, is in the README
- [ ] Every Android `.so` is 16 KB page-aligned (script output in the README)
- [ ] ADR 0029 is merged (status Accepted or Proposed with named open items) and records the 0.32 pin, its reasons and the upgrade policy
- [ ] The spike is outside both workspaces: `cargo xtask check` is unaffected

## Post-merge checks

None.

## Tests

- Rust unit tests in the spike for the wrappers and error mapping.
- Swift XCTest (macOS host with the simulator slice, or `swift test` against the macOS slice) and Kotlin JVM tests (JUnit) for the shared scenario.
- Cancellation and panic tests on both platforms.
- Fixtures: `fixtures/golden/` examples from B5c (read by path from the scripts; the library still takes bytes).

## Docs to update

- [ ] [shareable-libraries §9 L6](../../plans/shareable-libraries.md#9-to-dos): tick "`lamina-ffi` spike …; ADR 0029"; answer open questions 1 and 2 in [§11](../../plans/shareable-libraries.md#11-open-questions)
- [ ] `docs/decisions/README.md`, [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs); ADR 0002 gets a "see ADR 0029" note (no change to its decision)
- [ ] `CLAUDE.md` (spike entry and how to run it), `spikes/lamina-ffi/README.md`

## Risks and open questions

- **UniFFI pin (decided (project owner, 2026-10-29)): 0.32.** Zero-copy `&[u8]` input (a 50 MB PDF need not be copied into a `Vec<u8>`) and foreign traits matter more than React Native, which ADR 0002 defers. If the spike finds a blocker in 0.32, raise it with the owner rather than switching pins silently.
- **Kotlin cancellation in UniFFI 0.32 (to verify):** the research could not confirm whether abandoning a Kotlin `suspend` call drops the Rust future; the spike must observe it.
- **`panic=unwind` catching (to verify):** that UniFFI scaffolding catches panics only under `panic=unwind` is unverified; the panic test settles it.
- **Toolchain and runners.** Swift tests need macOS; Android needs the NDK. If the sandbox or CI lacks them, run on a maintainer's machine and record the environment; do not add runners in this PR.
- **androidx.pdf state (to verify, for ADR 0029's context):** 1.0.0-beta01 (26 August 2026, more likely) vs 1.0.0-alpha19; check the English release-notes page ([report: conflict to verify](../../research/libraries/shareable-libraries.md#conflict-to-verify-the-state-of-androidxpdf)).
- **Where derives live.** `cfg_attr(feature = "uniffi", derive(uniffi::Record))` on core types would add `uniffi` as an optional dependency of core crates; the portability check may need a rule for it. Wrapper types in `lamina-ffi` avoid that at the cost of duplication. ADR 0029 decides.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
