package dev.lamina

import dev.lamina.ffi.FfiCountJob
import dev.lamina.ffi.FfiDocument
import dev.lamina.ffi.FfiDropProbe
import dev.lamina.ffi.FfiException
import dev.lamina.ffi.FfiSaveMode
import dev.lamina.ffi.FfiSaveOptions
import dev.lamina.ffi.FfiSaveStep
import dev.lamina.ffi.InternalException
import dev.lamina.ffi.buildInfo
import dev.lamina.ffi.pageCountAsync
import dev.lamina.ffi.panicForTest
import dev.lamina.ffi.panicForTestFallible
import dev.lamina.ffi.runUntilCancelled
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import java.io.File
import java.nio.ByteBuffer
import java.security.MessageDigest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

/** SHA-256 of the scenario output from the Rust test (target/scenario-rust.pdf). */
const val SCENARIO_SHA256 = "12dc111c8a4c9b7f5b298caf19a84f17381387514b2ed51dafaf554d7feb2966"

class LaminaFfiTest {
    private val fixtures = File(System.getProperty("lamina.fixtures"))
    private val hello = File(fixtures, "golden/examples/hello.pdf").readBytes()

    private fun sha256(b: ByteArray) = MessageDigest.getInstance("SHA-256").digest(b).joinToString("") { "%02x".format(it) }

    @Test
    fun scenarioOpenReadEditSaveReopen() = runBlocking {
        println(buildInfo())
        LaminaDocument.open(hello).use { doc ->
            assertEquals(1, doc.pageCount())
            val page = doc.pages().single()
            println("page 0: ${page.width} x ${page.height} pt, id ${page.id}")
            val lines = doc.raw.pageText(0u)
            println("page 0 text: $lines")
            assertTrue(lines.isNotEmpty())
            doc.setTitle("From the FFI")
            assertEquals("From the FFI", doc.title())
            val saved = doc.saveBlocking(FfiSaveMode.INCREMENTAL)
            // Compared with Rust's and Swift's output for the same scenario (README table).
            assertEquals(SCENARIO_SHA256, sha256(saved.bytes), "same bytes as Rust and Swift")
            println("scenario sha256 ${sha256(saved.bytes)} (${saved.bytes.size} bytes, append at ${saved.appendOffset})")
            File("build/scenario-kotlin.pdf").writeBytes(saved.bytes)
            LaminaDocument.open(saved.bytes).use { reopened -> assertEquals("From the FFI", reopened.title()) }
        }
    }

    @Test
    fun zeroCopyOpenNeedsADirectBuffer() = runBlocking {
        val direct = ByteBuffer.allocateDirect(hello.size).put(hello).flip()
        LaminaDocument.open(direct).use { assertEquals(1, it.pageCount()) }
        assertFailsWith<IllegalArgumentException> { FfiDocument.openBorrowed(ByteBuffer.wrap(hello), null) }
        Unit
    }

    @Test
    fun errorsCarryTheContract() {
        val e = assertFailsWith<FfiException.Lamina> { FfiDocument.open("not a pdf".toByteArray(), null) }
        assertEquals("not-pdf", e.code)
        assertEquals("input", e.kind)
        assertTrue(e.json.contains("\"docsUrl\""))
    }

    @Test
    fun suspendSaveReportsProgressAndFinishes() = runBlocking {
        LaminaDocument.open(hello).use { doc ->
            doc.setTitle("job")
            var steps = 0
            val saved = doc.save(unitsPerStep = 1u) { steps++ }
            assertTrue(saved.bytes.isNotEmpty())
            assertTrue(steps > 0, "a 1-unit budget reports progress")
        }
    }

    @Test
    fun cancellingASuspendSaveThrowsCancellation() = runBlocking {
        LaminaDocument.open(hello).use { doc ->
            doc.setTitle("cancel me")
            val call = async { doc.save(unitsPerStep = 1u) { } }
            call.cancel()
            assertFailsWith<CancellationException> { call.await() }
            assertTrue(doc.raw.hasUnsavedChanges(), "a cancelled save leaves the edit unsaved")
        }
    }

    @Test
    fun cancellingTheCoroutineCancelsTheRustJobWithinOneStep() = runBlocking {
        val job = FfiCountJob(1_000_000u)
        val running =
            launch {
                while (!cancellableStep(Dispatchers.IO, onCancel = job::cancel) { job.step() }) Unit
            }
        while (job.done() < 10u) delay(1)
        running.cancel()
        running.join()
        val doneAtCancel = job.done()
        val e = assertFailsWith<FfiException.Lamina> { job.step() }
        assertEquals("cancelled", e.code)
        assertEquals(doneAtCancel, job.done(), "no unit runs after cancel")
    }

    @Test
    fun cancellingASuspendCallDropsTheRustFuture() = runBlocking {
        val probe = FfiDropProbe()
        val call = async(Dispatchers.Default) { runUntilCancelled(probe) }
        withTimeout(5_000) { while (probe.polls() < 3u) delay(1) }
        call.cancel()
        assertFailsWith<CancellationException> { call.await() }
        // The drop happens when the foreign side frees the future: give it a moment.
        withTimeout(5_000) { while (!probe.dropped()) delay(1) }
        println("kotlin: future dropped after cancel, polled ${probe.polls()} times")
        assertTrue(probe.dropped())
    }

    @Test
    fun asyncExportsWork() = runBlocking {
        FfiDocument.open(hello, null).use { assertEquals(1u, pageCountAsync(it)) }
    }

    @Test
    fun aPanicBecomesAnException() {
        val e = assertFailsWith<InternalException> { panicForTest() }
        println("kotlin: panic surfaced as InternalException(${e.message})")
        val f = assertFailsWith<InternalException> { panicForTestFallible() }
        println("kotlin: fallible panic surfaced as InternalException(${f.message})")
    }

    @Test
    fun aSaveJobOnAStaleDocumentFails() {
        FfiDocument.open(hello, null).use { doc ->
            doc.setInfo(dev.lamina.ffi.FfiInfoKey.SUBJECT, "a")
            val job = doc.saveJob(FfiSaveOptions(FfiSaveMode.AUTO, false))
            doc.setInfo(dev.lamina.ffi.FfiInfoKey.SUBJECT, "b")
            val e = assertFailsWith<FfiException.Lamina> { job.step(1u) }
            assertEquals("document-changed", e.code)
        }
    }

    @Test
    fun progressStepsAreProgress() {
        FfiDocument.open(hello, null).use { doc ->
            doc.setInfo(dev.lamina.ffi.FfiInfoKey.SUBJECT, "c")
            val step = doc.saveJob(FfiSaveOptions(FfiSaveMode.AUTO, false)).step(1u)
            assertTrue(step is FfiSaveStep.Progress)
        }
    }
}
