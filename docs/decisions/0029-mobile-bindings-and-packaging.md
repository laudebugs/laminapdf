# 0029 — Mobile bindings and packaging

- Status: Accepted (2026-10-30, when L6a's PR merges; every open item below is closed).
- Date: 2026-10-30

## Context

[ADR 0002 §5](0002-binding-strategy.md#decision) chose UniFFI for Swift and Kotlin, "pinned", later. [ADR 0028](0028-facade-rules-jobs-and-cancellation.md) wrote the façade rules with UniFFI's constraints in mind: `Arc` objects that are `Send + Sync`, no generics, jobs instead of callbacks, the lock in the edge. The owner pinned UniFFI to 0.32 on 2026-10-29 for its zero-copy byte arguments and foreign traits ([roadmap: decisions made](../plans/roadmap.md#decisions-made)).

The [`lamina-ffi` spike](../../spikes/lamina-ffi/README.md) (L6a) bound the B5c façade with UniFFI 0.32.2. What it found:
- **Kotlin:** proved on the desktop JVM.
- **Swift and Android:** proved on a Mac (2026-10-30): Swift XCTest on macOS and the iOS simulator, the Android libraries per ABI, 16 KB alignment and the AAR.

Background: the [shareable-libraries report](../research/libraries/shareable-libraries.md) and its [Rust core bridge](../research/libraries/notes/rust-core-bridge.md) and [mobile SDK](../research/libraries/notes/mobile-sdk.md) notes.

The measured findings:
- **Façade:** binds without changes.
- **Edge-only adaptations:** five, listed in the spike README.
- **Kotlin cancellation:** dropping a `suspend` call drops the Rust future.
- **Swift cancellation:** UniFFI 0.32's generated Swift ignores task cancellation: a cancelled task's Rust future is neither dropped nor stopped, and keeps being polled.
- **cargo-swift:** 0.11.1, the newest release, pins `uniffi_bindgen =0.31.1` and can't read 0.32 metadata.
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
   - **Locking:** a `Mutex<laminapdf::Document>` per handle. Jobs lock the job, then the document. `cancel()` takes no lock.
   - **Bytes in:** constructors take `&[u8]` (UniFFI 0.32 zero-copy), copied once into the owned document. In Kotlin that means a direct `ByteBuffer`, plus a `ByteArray` overload for convenience.
4. **An idiomatic, hand-written layer per language, on top of the generated bindings** (which are internal).
   - **Generated code stays internal:** the layer holds the generated handle privately (`internal` in Swift and Kotlin) and doesn't re-export the generated module. The spike's layer still uses the generated records (`FfiSaved`, `FfiPageInfo`) in its signatures; L6b's layer wraps them, so apps never depend on the generated module.
   - **Handles:** `Sendable` in Swift (`@unchecked`, since the Rust object locks itself) and `AutoCloseable` in Kotlin.
   - **Calls:** every call that locks the Rust document runs off the caller's thread: a detached task in Swift, the layer's dispatcher in Kotlin. Swift wraps job loops in `withTaskCancellationHandler { … } onCancel: { job.cancel() }`, and every async export the same way: since the generated Swift never cancels a Rust future, each async export takes a cancel token and the layer cancels it in `onCancel`. The layer never calls a generated async function without that wrapper. Async exports never block the foreign executor and never spin: while the document lock is held they return `Pending` with their waker registered, and releasing the lock or cancelling the token wakes them (the spike's `page_count_async` and `DocGuard`). Blocking calls run on a dispatch queue in Swift, not on the cooperative pool (a few threads), and on the layer's dispatcher in Kotlin. Kotlin runs each blocking step on the dispatcher and, on cancellation, calls `job.cancel()` and then waits non-cancellably for the step in flight, so its result is never dropped.
   - **A cancelled save that finished anyway:** a cancel that arrives during the save's last step loses the race, and the document is rebased onto the output. The layer must never lose those bytes. Swift returns them (a cancelled task still returns its value); Kotlin hands them to the call's `onSavedAfterCancel` callback, since a cancelled coroutine can't return (per call, so overlapping saves can't take each other's bytes).
   - **Cancellation:** lamina's `cancelled` maps to `CancellationError` in Swift and `CancellationException` in Kotlin.
   - **Progress:** reported through a callback in the layer, not across the FFI. `AsyncStream` and `Flow` come in L6b.
5. **Packaging.**
   - **iOS:** a Swift package with a binary target (an XCFramework of static libraries for `ios-arm64` and `ios-arm64-simulator`, plus `macos-arm64` for host tests), the generated bindings as a target, and the idiomatic target as the product. SwiftPM only, no CocoaPods (trunk is read-only from 2 December 2026).
   - **iOS build:** plain `cargo` per target, `uniffi-bindgen` 0.32 from the crate and `xcodebuild -create-xcframework`, as the spike's `build-ios.sh` does. Not cargo-swift: 0.11.1 pins UniFFI 0.31 and builds only `debug` or `release`.
   - **Android:** an AAR on Maven Central (`lamina-core`, later `lamina-compose`), built with the Android Gradle Plugin in L6b, with JNA as a dependency.
   - **ABIs:** `arm64-v8a` and `x86_64`. `armeabi-v7a` is measured and not shipped unless L6b finds demand.
   - **Minimum versions:** iOS 15 and Android API 24.
   - **16 KB pages:** every `.so` is linked with `-Wl,-z,max-page-size=16384` (NDK r28+), and a release check fails on any `LOAD` alignment other than `0x4000`.
6. **Profiles and generation.**
   - **`[profile.mobile-release]`:** inherits `release`, with `panic = "unwind"` (UniFFI catches panics only when unwinding), `lto = true`, `codegen-units = 1`, `opt-level = "s"` (`"z"` was not smaller under LTO) and `strip = true`.
   - **Bindings:** generated from the unstripped `release` build, since stripping removes UniFFI's metadata.
7. **Size budgets:**
   - **Measured (Mac, 2026-10-30, `mobile-release`):** Android `arm64-v8a` 2.45 MB raw / 0.97 MB gzip with `text` (1.51 / 0.70 without), `x86_64` 2.66 / 1.04 (1.68 / 0.75); iOS `arm64`, as a linked dylib (what the static library adds after dead-stripping), 2.04 / 0.85 (1.45 / 0.64). The spike README has every slice.
   - **Budgets (recommendation, about 12% over the measurement), checked by the release lane:**

     | Library | With `text` (raw / gzip) | Without `text` |
     | --- | --- | --- |
     | Android `arm64-v8a` `.so` | 2.75 MB / 1.10 MB | 1.70 MB / 0.80 MB |
     | Android `x86_64` `.so` | 3.00 MB / 1.20 MB | 1.90 MB / 0.85 MB |
     | iOS `arm64` (linked dylib) | 2.30 MB / 0.95 MB | 1.65 MB / 0.75 MB |

   - **Download size:** the static XCFramework is 95 MB zipped (three slices of 115 MB `.a`s before dead-stripping). L6b decides between that and a dynamic framework.
8. **CI for the product crate** (L6b):
   - **Pull requests:** a macOS job builds the XCFramework and runs `swift test`; a Linux job builds the Android ABIs, runs the alignment check and the Kotlin JVM tests.
   - **Release lane:** the iOS simulator and an Android emulator run.

## Open items (all closed)

1. ~~**UniFFI's licence (owner decision).**~~ **Decided 2026-10-30 (owner): an exception for the UniFFI crates.** All eight UniFFI crates are MPL-2.0. [`deny.toml`](../../deny.toml) now allows MPL-2.0 for exactly these crates (`uniffi`, `uniffi_bindgen`, `uniffi_core`, `uniffi_internal_macros`, `uniffi_macros`, `uniffi_meta`, `uniffi_pipeline`, `uniffi_udl`), with the reason written next to the entry. MPL-2.0 is file-level copyleft: lamina stays MIT OR Apache-2.0, the unmodified UniFFI sources stay available on crates.io, and the MPL-2.0 text ships with the mobile SDK. Whether the generated Swift and Kotlin files are covered by MPL-2.0 is unsettled. Recommendation (opinion): ship them unmodified, with the MPL-2.0 notice, which meets the obligations either way.
2. ~~**The Mac half of the spike.**~~ **Done 2026-10-30:** Swift XCTest passes on macOS and the iOS simulator (byte equality, job cancellation, the fallible panic); the Android libraries per ABI pass the 16 KB check; Kotlin passes on the macOS library; the AAR builds. Sizes are in §7. Two findings changed this ADR: Swift async cancellation (§4) and cargo-swift (§5).
3. ~~**androidx.pdf's release state.**~~ **Checked 2026-10-30:** 1.0.0-beta01 (26 August 2026) is the newest release, with no stable or RC; alpha18 backported reading and rendering to `minSdk` 28 ([release notes](https://developer.android.com/jetpack/androidx/releases/pdf)).

## Consequences

- The façade stays binding-agnostic, and the rules ADR 0028 wrote for UniFFI held without a change.
- **Swift panics:** an export added without a `Result` is a crash risk in Swift. L6b adds a test that every exported function in the generated Swift `throws`.
- **Passwords:** `@laminapdf/sdk` retries a failed password with its NFKC form; the spike passes the UTF-8 bytes as given. L6b's layer does the same retry (or the façade takes it over for every binding), so a password works the same on every surface.
- **Kotlin error field:** Kotlin users read `detail`, not `message`, on `FfiException.Lamina`. The idiomatic layer can expose a `LaminaException` with `message` set from `detail`.
- **Copies:** large inputs are copied once into Rust through the zero-copy `&[u8]` path (a direct `ByteBuffer` in Kotlin, from its position); a Kotlin `ByteArray` costs a second copy through a `RustBuffer`. Files of 50 MB and more should open through a host `ByteSource` (a mapped file) in L6b rather than through bytes.
- **Licence:** adds an MPL-2.0 dependency at the edge (the owner allowed it for the UniFFI crates, 2026-10-30).

## Alternatives considered

- **UniFFI 0.31 (for React Native through ubrn).** It loses zero-copy bytes and foreign traits. Rejected by the owner's pin.
- **UniFFI derives on core types behind a feature.** That means less duplication, but a binding dependency (MPL) in core crates and a portability-rule exception. Rejected.
- **cargo-swift as the iOS build.** Rejected for now: 0.11.1 pins UniFFI 0.31 (it fails on 0.32 metadata) and builds only `debug` or `release`. Revisit when a release supports 0.32.
- **The Android Gradle Plugin in the spike.** The spike zips the AAR itself (the SDK host was unreachable); the product uses AGP.
