# Spike: `lamina-ffi` (Round L6a)

**Question.** Does the `lamina` façade (B5c, ADR 0028) bind to Swift and Kotlin through UniFFI 0.32 without changes? How do cancellation and panics behave from each language, how big are the libraries, and what should ADR 0029 decide?

**Answer so far (2026-10-30).**
- **Binding:** yes, the façade binds without changes. Every adaptation is edge-only (list below).
- **Kotlin:** proved on the desktop JVM, 11 tests.
- **Swift and Android:** need Xcode and the Android NDK, which this research session couldn't reach. `scripts/run-on-mac.sh` runs that half in one go and writes the report its results paste into [§ Mac half](#mac-half-pending).
- **Licence:** UniFFI is **MPL-2.0**; the owner allowed it for the UniFFI crates on 2026-10-30, as an exception in [`deny.toml`](../../deny.toml) (see [licence](#licence-uniffi-is-mpl-20)).

Throwaway-quality code outside both workspaces (like `krilla-fit`). It depends on `crates/lamina` by path, and `cargo xtask check` doesn't see it.

## What is here

| Path | What |
| --- | --- |
| `src/lib.rs` | UniFFI exports over `lamina` (wrapper types only, nothing in core crates). See the export list below. |
| `kotlin/` | Gradle (Kotlin JVM) project: an idiomatic `suspend` layer (`LaminaDocument.kt`) and JUnit tests over JNA and the host library. `gradle aar` zips an AAR. |
| `swift/` | Swift package: a binary target (XCFramework), the generated bindings as their own target, and an idiomatic `async` layer (`LaminaDocument.swift`), with XCTests. |
| `scripts/` | `build-kotlin.sh` (bindings + JVM tests), `build-ios.sh` (XCFramework), `build-android.sh` (cargo-ndk), `check-alignment.sh` (16 KB), `size-report.sh`, `run-on-mac.sh` (all of the Mac half). |

The exports in `src/lib.rs`:
- **`FfiDocument`** (`Arc`, a `Mutex<lamina::Document>` inside): `open`, `open_borrowed(&[u8])`, `page_count`, `page_infos`, `page_text`, `info`, `set_info`, `undo`, `warnings`, `save`, `save_job`.
- **Jobs:** `FfiSaveJob` with `step(units)` and `cancel()`, and `FfiCancelToken`.
- **Errors:** `FfiError`, carrying ADR 0026's fields.
- **Test probes:**
  - `FfiCountJob`: a toy job long enough to cancel half-way.
  - `FfiDropProbe` with `run_until_cancelled`: is the Rust future dropped?
  - `panic_for_test` and `panic_for_test_fallible`.
  - `page_count_async`.

Pins: `uniffi =0.32.2` (owner decision 2026-10-29), cargo-swift 0.11.1, cargo-ndk 4.1.2, Android NDK r28c, JNA 5.17.0, kotlinx-coroutines 1.10.2.

## Running it

```sh
cargo test                                # Rust: the scenario, error mapping, jobs, cancellation
scripts/build-kotlin.sh                   # host library + Kotlin bindings + JVM tests (JDK 17+, Gradle 8)
scripts/run-on-mac.sh                     # Apple silicon + Xcode: everything below, report in target/mac-report.md
```

`run-on-mac.sh` runs, in order:
- **Apple:**
  - `build-ios.sh`, then `swift test` (macOS slice) and `xcodebuild test` on an iOS simulator.
  - The no-text libraries.
  - The cargo-swift probe.
- **Android:**
  - The NDK, downloaded to `~/.cache/lamina-ffi` unless `ANDROID_NDK_HOME` is set.
  - `build-android.sh` with and without `text`, then `check-alignment.sh`.
- **Kotlin:** the tests again on the macOS library, then the AAR.
- **Sizes:** `size-report.sh`.

The rustup targets the scripts add are not added to the root `rust-toolchain.toml`: `aarch64-apple-ios-sim`, `aarch64-apple-darwin`, `x86_64-linux-android` and `armv7-linux-androideabi`.

## Findings

### The façade binds without changes

None of B5c's façade had to change. ADR 0028's rules held:
- Owned records, `u32` counts and bytes as `Vec<u8>` map straight onto UniFFI records.
- With no lifetimes or generics on `Document`, it maps onto an `Arc<FfiDocument>`.
- Jobs replace callbacks.

`Document` is `Send + !Sync`, so the lock lives in the edge, as [ADR 0028 §2](../../docs/decisions/0028-facade-rules-jobs-and-cancellation.md) planned: a `Mutex<Document>`. `SaveJob::step(&mut doc, budget)` locks the job and then the document. `cancel()` takes no lock, so it works while another thread is inside a step.

Edge-only adaptations UniFFI forced, each recorded in ADR 0029:

| # | Adaptation | The rule that forced it |
| --- | --- | --- |
| 1 | `Step<Saved>` becomes a concrete `FfiSaveStep` enum | UniFFI has no generics (ADR 0028 F3 already says one record per job) |
| 2 | ADR 0026's `message` field is called `detail` in `FfiError` | A field named `message` clashes with `Throwable.message` in the generated Kotlin exception, and the code doesn't compile. UniFFI 0.32 doesn't rename it. |
| 3 | `page_text` (feature `text`) lives in its own `#[cfg(feature = "text")] #[uniffi::export] impl` block | A `#[cfg]` on one method inside an exported block is not honoured: the scaffolding still references it, and the build without `text` fails |
| 4 | `#![allow(clippy::result_large_err)]` | UniFFI error variants hold their fields by value (no `Box`). The error crosses the FFI serialised. |
| 5 | Every export that locks or calls into `lamina` returns `Result`. The spike's infallible exports, all trivial, are: handle constructors that only allocate (`FfiCancelToken::new`, `FfiCountJob::new`, `FfiDropProbe::new`), atomic accessors (`cancel`, `is_cancelled`, `done`, `dropped`, `polls`), `build_info`, the test future `run_until_cancelled`, and the deliberate `panic_for_test`. | Swift calls infallible exports with `try!`, so a panic in one traps the app (see panics) |

### Zero-copy `&[u8]` input works, with one copy left

`open_borrowed(bytes: &[u8])` compiles and runs.
- **Swift:** takes `Data` and lowers it inside `Data.withUnsafeBytes`.
- **Kotlin:** takes a `java.nio.ByteBuffer`, which must be **direct** (`ByteBuffer.allocateDirect`). A heap buffer (`ByteBuffer.wrap(array)`) throws `IllegalArgumentException` (test `zeroCopyOpenNeedsADirectBuffer`).
- **Remaining copy:** `Document::open_bytes` owns its `Vec<u8>`, and the document outlives the call, so the edge still copies the bytes once (`to_vec`). Zero-copy saves the foreign-to-`RustBuffer` copy, not this one.

Removing the last copy would need a `ByteSource` over foreign memory that outlives the call. That isn't safe through UniFFI, and a mapped file is a better route for 50 MB+ files (a host `ByteSource`, L6b).

### Cancellation

- **Jobs:** cancelling stops a job within one step on Kotlin, and the next step returns `cancelled` (tests `cancellingTheCoroutineCancelsTheRustJobWithinOneStep` and `cancellingASuspendSaveThrowsCancellation`).
  - The working Kotlin pattern (`cancellableStep` in `LaminaDocument.kt`): run each blocking step on the dispatcher behind a `CompletableDeferred`; on cancellation call `job.cancel()`, then wait non-cancellably for that step and keep its value. A plain `suspendCancellableCoroutine` with `invokeOnCancellation` resumes at once and drops a step that finished the save.
  - A first version used a completion handler disposed in `finally`, and it never fired. The handle was disposed before the coroutine finished cancelling.
  - After a cancel, the layer waits (non-cancellably) for the step already running, at most one. Without that, a step that finished the save was dropped: Rust had saved and rebased, but the bytes never reached anyone.
- **A cancelled save can still finish.** If the save completes during the step that was running when the cancel arrived, the document is already rebased onto the new output.
  - **Kotlin:** a cancelled coroutine can't return a value, so the layer keeps the bytes in `LaminaDocument.lastSaved`. On `hello.pdf` (a few steps) the test usually takes this path.
  - **Swift:** a cancelled task still returns its value, so `save()` simply returns the bytes.
  - **Product layer:** must expose the same guarantee: never neither the bytes nor the unsaved edit.
- **Kotlin async (UniFFI 0.32): verified.** Cancelling the coroutine of a `suspend` export **drops the Rust future**, and our `Drop` runs (test `cancellingASuspendCallDropsTheRustFuture`). This answers the open question in [shareable-libraries §11](../../docs/plans/shareable-libraries.md#11-open-questions).
- **Swift:** the matching tests are in `LaminaTests.swift`: job cancellation, a cancelled `save()` (`testCancellingASaveNeverLosesBoth`) and the dropped future, with `withTaskCancellationHandler` and `Task.cancel()`. Results pending the Mac run.

### Panics

- **Kotlin:** with `panic = "unwind"` (both `release` and `mobile-release`), a panic surfaces as `InternalException` from both an infallible and a fallible export. The JVM keeps running.
- **Swift:** the generated code calls **infallible** exports with `try!`, so a panic in one traps the process. A panic in a **fallible** export becomes `UniffiInternalError.rustPanic`; that test is pending the Mac run.
- **Product rule:** every export returns `Result`, and mobile profiles never use `panic = "abort"`.

### Bindings must come from an unstripped build

`strip = true` (in `mobile-release`) removes the symbols `uniffi-bindgen --library` reads its metadata from ("No UniFFI metadata found"). The scripts generate bindings from the `release` build and ship the stripped `mobile-release` libraries.

### Same bytes everywhere

The scenario gives the same output, SHA-256 `12dc111c…2966` (1,774 bytes, append offset 1,395), from Rust and from Kotlin. The scenario: open `fixtures/golden/examples/hello.pdf`, read page count, sizes and text, `set_info(title)`, incremental save, reopen. The Rust test compares the FFI path with the façade called directly and asserts the hash; the Kotlin and Swift tests assert the same hash.

### Sizes (Linux x86_64 host; Android and Apple pending)

| Library (`mobile-release`: `opt-level = "s"`, LTO, 1 CGU, stripped) | Raw | gzip -9 |
| --- | --- | --- |
| `liblamina_ffi.so`, with `text` | 2.64 MB | 1.03 MB |
| `liblamina_ffi.so`, without `text` | 1.65 MB | 0.74 MB |
| Same with `opt-level = "z"` (with `text`) | 2.69 MB | 1.02 MB |

- **`text` feature:** fonts, shaping, ICU segmentation and extraction add about 1 MB raw, or about 0.3 MB compressed.
- **`opt-level = "z"`:** not smaller than `"s"` under LTO, so `mobile-release` stays at `"s"`.
- **JNA:** its AAR (`net.java.dev.jna:jna:5.17.0@aar`, with its own native libraries per ABI) adds 510 KB to an Android app.

### Licence: UniFFI is MPL-2.0

`cargo deny check licenses` with the repo's [`deny.toml`](../../deny.toml) fails on every UniFFI crate: `uniffi`, `uniffi_core`, `uniffi_macros`, `uniffi_internal_macros`, `uniffi_meta`, `uniffi_bindgen`, `uniffi_pipeline` and `uniffi_udl` 0.32.2, all MPL-2.0.

MPL-2.0 is file-level copyleft, not GPL. It lets lamina stay MIT OR Apache-2.0 and ship binaries, as long as the MPL files' source stays available (unmodified crates.io sources qualify) and their notices ship. But it is not on ADR 0019's allow-list. And the generated Swift and Kotlin come from MPL-licensed templates in `uniffi_bindgen`; whether that makes the generated files MPL as well is not settled here. **Decided 2026-10-30 (owner):** `deny.toml` now allows MPL-2.0 for exactly these eight crates, and `cargo deny` passes for the spike. The generated files ship unmodified with the MPL-2.0 notice (ADR 0029, item 1).

### Not yet verified (the Mac half)

- XCFramework build and `swift test`.
- Byte equality from Swift.
- Swift cancellation and panic behaviour.
- iOS simulator tests.
- cargo-swift 0.11.1, which builds only the `debug` or `release` profile, not `mobile-release`.
- Android libraries per ABI, 16 KB alignment and the AAR.
- Apple and Android sizes.
- The androidx.pdf release state (ADR 0029 context), not checked: `developer.android.com` and `dl.google.com` are blocked from the research session.

## Mac half (pending)

Run `scripts/run-on-mac.sh` on an Apple-silicon Mac and paste `target/mac-report.md` here. Then fill in ADR 0029's size budgets and close its open items.
