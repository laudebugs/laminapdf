import Foundation
@_exported import LaminaFFI

/// The hand-written idiomatic layer over the generated bindings (ADR 0029: generated code is
/// internal, this is the API). Every call that locks the Rust document runs off the caller's
/// thread (`Task.detached`), so a long step never blocks the main actor. Handles are
/// `Sendable` because the Rust object locks itself; long work is a job stepped in a loop, and
/// cancelling the Swift task cancels the Rust job through its token
/// (`withTaskCancellationHandler`). Unlike Kotlin, a Swift task returns its value even when
/// cancelled, so a save whose last step already finished still hands back its bytes.
public final class LaminaDocument: @unchecked Sendable {
    public let raw: FfiDocument

    private init(raw: FfiDocument) { self.raw = raw }

    /// Runs a blocking Rust call on a detached task at the caller's priority (detached so it
    /// never runs on the main actor; one task per call, and one for a whole job loop).
    static func offCaller<T: Sendable>(_ body: @escaping @Sendable () throws -> T) async throws -> T {
        try await Task.detached(priority: Task.currentPriority) { try body() }.value
    }

    /// Opens a document. `Data` crosses as UniFFI 0.32's zero-copy `&[u8]`: read in place for
    /// the call; Rust copies it once to own it.
    public static func open(_ bytes: Data, password: String? = nil) async throws -> LaminaDocument {
        LaminaDocument(raw: try await offCaller { try FfiDocument.openBorrowed(bytes: bytes, password: password) })
    }

    public func pageCount() async throws -> Int {
        let raw = self.raw
        return Int(try await Self.offCaller { try raw.pageCount() })
    }

    public func pages() async throws -> [FfiPageInfo] {
        let raw = self.raw
        return try await Self.offCaller { try raw.pageInfos() }
    }

    public func title() async throws -> String? {
        let raw = self.raw
        return try await Self.offCaller { try raw.info().title }
    }

    public func setTitle(_ title: String?) async throws {
        let raw = self.raw
        try await Self.offCaller { try raw.setInfo(key: .title, value: title) }
    }

    /// Saves as a job of `unitsPerStep` units per step; cancelling the task cancels the Rust
    /// job and this throws `CancellationError` (unless the save had already finished).
    /// `onProgress` runs on the job's background task: hop to the main actor to update UI.
    public func save(
        mode: FfiSaveMode = .auto,
        unitsPerStep: UInt32 = 256,
        onProgress: @escaping @Sendable (UInt32, UInt32?, String) -> Void = { _, _, _ in }
    ) async throws -> FfiSaved {
        let raw = self.raw
        let job = try await Self.offCaller { try raw.saveJob(options: FfiSaveOptions(mode: mode, appendedOnly: false)) }
        // The whole loop runs in one detached task; cancelling this task cancels the Rust job,
        // whose next step then fails with `cancelled` (mapped to CancellationError). A step that
        // had already finished the save returns its bytes: a cancelled Swift task still returns.
        return try await withTaskCancellationHandler {
            try await Self.offCaller {
                while true {
                    switch try Self.mapCancelled({ try job.step(units: unitsPerStep) }) {
                    case let .done(saved):
                        return saved
                    case let .progress(done, total, phase):
                        onProgress(done, total, phase)
                    }
                }
            }
        } onCancel: {
            job.cancel()
        }
    }

    /// Turns lamina's `cancelled` error into Swift's `CancellationError`.
    public static func mapCancelled<T>(_ body: () throws -> T) throws -> T {
        do {
            return try body()
        } catch let FfiError.Lamina(code, _, _, _, _, _) where code == "cancelled" {
            throw CancellationError()
        }
    }
}

/// Cancellation for async exports. UniFFI 0.32's generated Swift ignores task cancellation: it
/// polls the Rust future until it completes and frees it only then, so `Task.cancel()` alone
/// never stops or drops it. The layer wraps every async export: `withTaskCancellationHandler`
/// tells Rust to stop (here the probe; a token in product code), and the future then finishes.
public func runUntilCancelledOrTaskCancelled(probe: FfiDropProbe) async {
    await withTaskCancellationHandler {
        await runUntilCancelled(probe: probe)
    } onCancel: {
        probe.cancel()
    }
}
