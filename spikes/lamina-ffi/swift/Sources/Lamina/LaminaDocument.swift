import Foundation
@_exported import LaminaFFI

/// The hand-written idiomatic layer over the generated bindings (ADR 0029: generated code is
/// internal, this is the API). Handles are `Sendable` because the Rust object locks itself;
/// long work is a job stepped in a loop, and cancelling the Swift task cancels the Rust job
/// through its token (`withTaskCancellationHandler`).
public final class LaminaDocument: @unchecked Sendable {
    public let raw: FfiDocument

    /// Opens a document. `Data` crosses as UniFFI 0.32's zero-copy `&[u8]`: read in place for
    /// the call; Rust copies it once to own it.
    public init(_ bytes: Data, password: String? = nil) throws {
        raw = try FfiDocument.openBorrowed(bytes: bytes, password: password)
    }

    public func pageCount() throws -> Int { Int(try raw.pageCount()) }

    public func pages() throws -> [FfiPageInfo] { try raw.pageInfos() }

    public func title() throws -> String? { try raw.info().title }

    public func setTitle(_ title: String?) throws { try raw.setInfo(key: .title, value: title) }

    /// Saves as a job of `unitsPerStep` units per step; cancelling the task cancels the Rust
    /// job and this throws `CancellationError`.
    public func save(
        mode: FfiSaveMode = .auto,
        unitsPerStep: UInt32 = 256,
        onProgress: @Sendable (UInt32, UInt32?, String) -> Void = { _, _, _ in }
    ) async throws -> FfiSaved {
        let job = try raw.saveJob(options: FfiSaveOptions(mode: mode, appendedOnly: false))
        return try await withTaskCancellationHandler {
            while true {
                try Task.checkCancellation()
                switch try Self.mapCancelled({ try job.step(units: unitsPerStep) }) {
                case let .done(saved):
                    return saved
                case let .progress(done, total, phase):
                    onProgress(done, total, phase)
                    await Task.yield()
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
