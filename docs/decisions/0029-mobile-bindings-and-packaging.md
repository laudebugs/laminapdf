# 0029 — Mobile bindings and packaging

- Status: Proposed. Open items 2 and 3 below must close before acceptance (item 1 is decided).
- Date: 2026-10-30

## Context

[ADR 0002 §5](0002-binding-strategy.md#decision) chose UniFFI for Swift and Kotlin, "pinned", later. [ADR 0028](0028-facade-rules-jobs-and-cancellation.md) wrote the façade rules with UniFFI's constraints in mind: `Arc` objects that are `Send + Sync`, no generics, jobs instead of callbacks, the lock in the edge. The owner pinned UniFFI to 0.32 on 2026-10-29 for its zero-copy byte arguments and foreign traits ([roadmap: decisions made](../plans/roadmap.md#decisions-made)).

The [`lamina-ffi` spike](../../spikes/lamina-ffi/README.md) (L6a) bound the B5c façade with UniFFI 0.32.2. What it found:
- **Kotlin:** proved on the desktop JVM.
- **Swift and Android:** builds and scripts written; measurements pending a Mac run.

Background: the [shareable-libraries report](../research/libraries/shareable-libraries.md) and its [Rust core bridge](../research/libraries/notes/rust-core-bridge.md) and [mobile SDK](../research/libraries/notes/mobile-sdk.md) notes.

The measured findings:
- **Façade:** binds without changes.
- **Edge-only adaptations:** five, listed in the spike README.
- **Kotlin cancellation:** dropping a `suspend` call drops the Rust future.
- **Panics:** a panic in an infallible export traps in Swift (`try!`) but is an exception in Kotlin.
- **Stripping:** a stripped library carries no UniFFI metadata.
- **Licence:** UniFFI is MPL-2.0.

## Decision

1. **UniFFI `=0.32.2`, pinned exactly.**
   - **Upgrade policy:** upgrades are deliberate PRs that regenerate both bindings, diff them, and run the spike's scenario on Swift and Kotlin, with byte-equal output.
   - **Patch releases:** take the same path.
   - **React Native and Kotlin Multiplatform:** React Native (ubrn supports only `=0.31`) and Kotlin Multiplatform (Gobley) are re-evaluated when scheduled, not by moving the pin.
2. **One edge crate, `crates/lamina-ffi`** (layer `edge`).
   - **What goes in it:** binding code only, meaning UniFFI wrapper types and conversions over `lamina`. No PDF logic.
   - **No UniFFI in core crates:** derives live on wrapper types in `lamina-ffi`, not as `cfg_attr(feature = "uniffi", derive(…))` on core types. Core crates take no UniFFI dependency (none of the portability rules change, and the MPL code stays at the edge). The cost is duplicated record definitions, which a test keeps in step with the façade.
3. **Rules for the edge crate**, from the spike:
   - **Errors:** every export that locks or calls into `lamina` returns `Result<_, FfiError>`, because Swift calls infallible exports with `try!`, so a panic would trap the app. Exports that can't fail except by running out of memory may stay infallible: handle constructors that only allocate, atomic accessors and constant strings (the spike's full list, test probes included, is in its README). L6b's test lists them.
   - **Error fields:** `FfiError` carries ADR 0026's fields as `code`, `kind`, `detail` (ADR 0026's `message`), `hint`, `docs_url` and `json`. No field may be named `message`: it clashes with `Throwable.message` in Kotlin.
   - **Feature-gated exports:** each goes in its own `#[cfg(feature = …)] #[uniffi::export] impl` block. A `cfg` on one method inside an exported block is not honoured.
   - **Records:** one concrete record or enum per job step (`FfiSaveStep`), per ADR 0028 F3.
   - **Locking:** a `Mutex<lamina::Document>` per handle. Jobs lock the job, then the document. `cancel()` takes no lock.
   - **Bytes in:** constructors take `&[u8]` (UniFFI 0.32 zero-copy), copied once into the owned document. In Kotlin that means a direct `ByteBuffer`, plus a `ByteArray` overload for convenience.
4. **An idiomatic, hand-written layer per language, on top of the generated bindings** (which are internal).
   - **Handles:** `Sendable` in Swift (`@unchecked`, since the Rust object locks itself) and `AutoCloseable` in Kotlin.
   - **Calls:** every call that locks the Rust document runs off the caller's thread: a detached task in Swift, the layer's dispatcher in Kotlin. Swift wraps job loops in `withTaskCancellationHandler { … } onCancel: { job.cancel() }`. Kotlin runs each blocking step on the dispatcher and, on cancellation, calls `job.cancel()` and then waits non-cancellably for the step in flight, so its result is never dropped.
   - **A cancelled save that finished anyway:** a cancel that arrives during the save's last step loses the race, and the document is rebased onto the output. The layer must never lose those bytes. Swift returns them (a cancelled task still returns its value); Kotlin keeps them in `lastSaved`, since a cancelled coroutine can't return.
   - **Cancellation:** lamina's `cancelled` maps to `CancellationError` in Swift and `CancellationException` in Kotlin.
   - **Progress:** reported through a callback in the layer, not across the FFI. `AsyncStream` and `Flow` come in L6b.
5. **Packaging.**
   - **iOS:** a Swift package with a binary target (an XCFramework of static libraries for `ios-arm64` and `ios-arm64-simulator`, plus `macos-arm64` for host tests), the generated bindings as a target, and the idiomatic target as the product. SwiftPM only, no CocoaPods (trunk is read-only from 2 December 2026).
   - **Android:** an AAR on Maven Central (`lamina-core`, later `lamina-compose`), built with the Android Gradle Plugin in L6b, with JNA as a dependency.
   - **ABIs:** `arm64-v8a` and `x86_64`. `armeabi-v7a` is measured and not shipped unless L6b finds demand.
   - **Minimum versions:** iOS 15 and Android API 24.
   - **16 KB pages:** every `.so` is linked with `-Wl,-z,max-page-size=16384` (NDK r28+), and a release check fails on any `LOAD` alignment other than `0x4000`.
6. **Profiles and generation.**
   - **`[profile.mobile-release]`:** inherits `release`, with `panic = "unwind"` (UniFFI catches panics only when unwinding), `lto = true`, `codegen-units = 1`, `opt-level = "s"` (`"z"` was not smaller under LTO) and `strip = true`.
   - **Bindings:** generated from the unstripped `release` build, since stripping removes UniFFI's metadata.
7. **Size budgets:**
   - **Measured so far, Linux host:** `mobile-release` is 2.64 MB raw and 1.03 MB gzip with `text`, or 1.65 MB and 0.74 MB without.
   - **Still to set:** the budgets per ABI and per slice, from the Mac run (open item 2).
8. **CI for the product crate** (L6b):
   - **Pull requests:** a macOS job builds the XCFramework and runs `swift test`; a Linux job builds the Android ABIs, runs the alignment check and the Kotlin JVM tests.
   - **Release lane:** the iOS simulator and an Android emulator run.

## Open items (before acceptance)

1. ~~**UniFFI's licence (owner decision).**~~ **Decided 2026-10-30 (owner): an exception for the UniFFI crates.** All eight UniFFI crates are MPL-2.0. [`deny.toml`](../../deny.toml) now allows MPL-2.0 for exactly these crates (`uniffi`, `uniffi_bindgen`, `uniffi_core`, `uniffi_internal_macros`, `uniffi_macros`, `uniffi_meta`, `uniffi_pipeline`, `uniffi_udl`), with the reason written next to the entry. MPL-2.0 is file-level copyleft: lamina stays MIT OR Apache-2.0, the unmodified UniFFI sources stay available on crates.io, and the MPL-2.0 text ships with the mobile SDK. Whether the generated Swift and Kotlin files are covered by MPL-2.0 is unsettled. Recommendation (opinion): ship them unmodified, with the MPL-2.0 notice, which meets the obligations either way.
2. **The Mac half of the spike.** Before this ADR is accepted, run `spikes/lamina-ffi/scripts/run-on-mac.sh` and record the results. They cover:
   - Swift XCTest with byte equality, cancellation and the fallible panic, plus the iOS simulator.
   - The cargo-swift 0.11.1 probe. It builds only `debug` or `release`, so the product may build with plain `cargo` and `xcodebuild -create-xcframework`, as the spike does.
   - Android libraries per ABI, 16 KB alignment, and the AAR.
   - Apple and Android sizes, so §7's budgets can be set.
3. **androidx.pdf's release state:** 1.0.0-beta01 or alpha19 ([report: conflict to verify](../research/libraries/shareable-libraries.md#conflict-to-verify-the-state-of-androidxpdf)). The research session couldn't reach `developer.android.com`.

## Consequences

- The façade stays binding-agnostic, and the rules ADR 0028 wrote for UniFFI held without a change.
- **Swift panics:** an export added without a `Result` is a crash risk in Swift. L6b adds a test that every exported function in the generated Swift `throws`.
- **Kotlin error field:** Kotlin users read `detail`, not `message`, on `FfiException.Lamina`. The idiomatic layer can expose a `LaminaException` with `message` set from `detail`.
- **Copies:** large inputs are copied once into Rust. Files of 50 MB and more should open through a host `ByteSource` (a mapped file) in L6b rather than through bytes.
- **Licence:** adds an MPL-2.0 dependency at the edge (the owner allowed it for the UniFFI crates, 2026-10-30).

## Alternatives considered

- **UniFFI 0.31 (for React Native through ubrn).** It loses zero-copy bytes and foreign traits. Rejected by the owner's pin.
- **UniFFI derives on core types behind a feature.** That means less duplication, but a binding dependency (MPL) in core crates and a portability-rule exception. Rejected.
- **cargo-swift as the iOS build.** It is convenient, but it doesn't build a custom profile. To be decided by open item 2.
- **The Android Gradle Plugin in the spike.** The spike zips the AAR itself (the SDK host was unreachable); the product uses AGP.
