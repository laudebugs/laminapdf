import CryptoKit
import Foundation
import XCTest
@testable import Lamina
import LaminaFFI

/// SHA-256 of the scenario's output: `SCENARIO_SHA256` in src/lib.rs's Rust test; Kotlin asserts
/// the same value.
let scenarioSHA256 = "12dc111c8a4c9b7f5b298caf19a84f17381387514b2ed51dafaf554d7feb2966"

final class LaminaTests: XCTestCase {
    /// fixtures/ at the repository root, found from this file's path.
    static let fixtures = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("fixtures")
    let hello = try! Data(contentsOf: fixtures.appendingPathComponent("golden/examples/hello.pdf"))

    func sha256(_ d: Data) -> String { SHA256.hash(data: d).map { String(format: "%02x", $0) }.joined() }

    func testScenarioOpenReadEditSaveReopen() async throws {
        print(buildInfo())
        let doc = try await LaminaDocument.open(hello)
        let count = try await doc.pageCount()
        XCTAssertEqual(count, 1)
        let pages = try await doc.pages()
        let page = try XCTUnwrap(pages.first)
        print("swift: page 0: \(page.width) x \(page.height) pt, id \(page.id)")
        let lines = try doc.raw.pageText(index: 0)
        print("swift: page 0 text: \(lines)")
        XCTAssertFalse(lines.isEmpty)
        try await doc.setTitle("From the FFI")
        let title = try await doc.title()
        XCTAssertEqual(title, "From the FFI")
        let saved = try doc.raw.save(options: FfiSaveOptions(mode: .incremental, appendedOnly: false), cancel: FfiCancelToken())
        XCTAssertEqual(sha256(saved.bytes), scenarioSHA256, "same bytes as Rust and Kotlin")
        print("swift: scenario sha256 \(sha256(saved.bytes)) (\(saved.bytes.count) bytes)")
        let reopened = try await LaminaDocument.open(saved.bytes).title()
        XCTAssertEqual(reopened, "From the FFI")
    }

    func testOwnedOpenToo() throws {
        XCTAssertEqual(try FfiDocument.open(bytes: hello, password: nil).pageCount(), 1)
    }

    func testErrorsCarryTheContract() {
        XCTAssertThrowsError(try FfiDocument.openBorrowed(bytes: Data("not a pdf".utf8), password: nil)) { error in
            guard case let FfiError.Lamina(code, kind, _, _, _, json) = error else { return XCTFail("\(error)") }
            XCTAssertEqual(code, "not-pdf")
            XCTAssertEqual(kind, "input")
            XCTAssertTrue(json.contains("\"docsUrl\""))
        }
    }

    func testAsyncSaveReportsProgressAndFinishes() async throws {
        let doc = try await LaminaDocument.open(hello)
        try await doc.setTitle("job")
        let counter = Counter()
        let saved = try await doc.save(unitsPerStep: 1) { _, _, _ in counter.add() }
        XCTAssertFalse(saved.bytes.isEmpty)
        XCTAssertGreaterThan(counter.value, 0)
    }

    func testCancellingASaveNeverLosesBoth() async throws {
        let doc = try await LaminaDocument.open(hello)
        try await doc.setTitle("cancel me")
        let started = Counter()
        let task = Task { try await doc.save(unitsPerStep: 1) { _, _, _ in started.add() } }
        while started.value == 0 { try await Task.sleep(nanoseconds: 100_000) }
        task.cancel()
        switch await task.result {
        case let .success(saved):
            // The last step had already finished: the bytes come back and the document is saved.
            XCTAssertFalse(saved.bytes.isEmpty)
            XCTAssertFalse(try doc.raw.hasUnsavedChanges())
            print("swift: the cancelled save had already finished and returned its bytes")
        case let .failure(error):
            XCTAssertTrue(error is CancellationError, "\(error)")
            XCTAssertTrue(try doc.raw.hasUnsavedChanges(), "a stopped save leaves the edit unsaved")
        }
    }

    func testCancellingTheTaskCancelsTheRustJobWithinOneStep() async throws {
        let job = FfiCountJob(total: 1_000_000)
        let task = Task.detached {
            try await withTaskCancellationHandler {
                while try !LaminaDocument.mapCancelled({ try job.step() }) { await Task.yield() }
            } onCancel: {
                job.cancel()
            }
        }
        while job.done() < 10 { try await Task.sleep(nanoseconds: 1_000_000) }
        task.cancel()
        do {
            try await task.value
            XCTFail("expected cancellation")
        } catch is CancellationError {}
        let doneAtCancel = job.done()
        XCTAssertThrowsError(try job.step()) { error in
            guard case let FfiError.Lamina(code, _, _, _, _, _) = error else { return XCTFail("\(error)") }
            XCTAssertEqual(code, "cancelled")
        }
        XCTAssertEqual(job.done(), doneAtCancel, "no unit runs after cancel")
    }

    func testCancellingAnAsyncCallDropsTheRustFuture() async throws {
        let probe = FfiDropProbe()
        let task = Task.detached { await runUntilCancelled(probe: probe) }
        while probe.polls() < 3 { try await Task.sleep(nanoseconds: 1_000_000) }
        task.cancel()
        _ = await task.result
        for _ in 0..<5000 where !probe.dropped() { try await Task.sleep(nanoseconds: 1_000_000) }
        print("swift: future dropped after cancel = \(probe.dropped()), polled \(probe.polls()) times")
        XCTAssertTrue(probe.dropped())
    }

    func testAsyncExportsWork() async throws {
        let n = try await pageCountAsync(doc: try FfiDocument.open(bytes: hello, password: nil))
        XCTAssertEqual(n, 1)
    }

    func testAPanicInAFallibleExportBecomesAnError() {
        XCTAssertThrowsError(try panicForTestFallible()) { error in
            print("swift: fallible panic surfaced as \(error)")
        }
    }
}

final class Counter: @unchecked Sendable {
    private let lock = NSLock()
    private var n = 0
    func add() { lock.lock(); n += 1; lock.unlock() }
    var value: Int { lock.lock(); defer { lock.unlock() }; return n }
}
